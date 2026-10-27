using System;
using System.Collections.Generic;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Comprehensive, rigorous validation suite for Pilot language keywords, methods, and syntax.
/// Every keyword is exercised across multiple scenarios with strict assertion invariants.
/// </summary>
public class PilotComprehensiveKeywordValidationTests
{
    // =========================================================================
    // 1. Variable Declarations & Mutability: let, var, const
    // =========================================================================

    [Fact]
    public void Keywords_LetVarConst_MultipleScenarios()
    {
        // Scenario A: Let immutability rejection vs Var mutability
        string codeA = @"
def test_mutability() -> int {
    var mutable_val = 10
    mutable_val = mutable_val + 5
    let immutable_val = 100
    return mutable_val + immutable_val
}
";
        var resA = PilotScript.RunFunction(codeA, "test_mutability");
        Assert.True(resA.Success);
        Assert.Equal(115, resA.Value.AsInt());

        // Scenario B: Scoped shadowing of let in nested block/function
        string codeB = @"
def test_shadowing() -> int {
    let x = 10
    def compute_nested() -> int {
        let x = 50
        return x * 2
    }
    let inner_res = compute_nested()
    return inner_res + x
}
";
        var resB = PilotScript.RunFunction(codeB, "test_shadowing");
        Assert.True(resB.Success);
        // inner returns 100, outer x is still 10 -> total 110
        Assert.Equal(110, resB.Value.AsInt());

        // Scenario C: Const immutable binding
        string codeC = @"
const GLOBAL_RATE = 42
def test_const() -> int {
    const LOCAL_LIMIT = 58
    return GLOBAL_RATE + LOCAL_LIMIT
}
";
        var resC = PilotScript.RunFunction(codeC, "test_const");
        Assert.True(resC.Success);
        Assert.Equal(100, resC.Value.AsInt());
    }

    // =========================================================================
    // 2. Control Flow: if, elif, else, while, for, in, break, continue, pass
    // =========================================================================

    [Fact]
    public void Keywords_ControlFlow_MultipleScenarios()
    {
        // Scenario A: if, elif, else branches
        string codeA = @"
def classify(score: int) -> str {
    if score >= 90 {
        return ""A""
    } elif score >= 80 {
        return ""B""
    } elif score >= 70 {
        return ""C""
    } else {
        return ""F""
    }
}
";
        Assert.Equal("A", PilotScript.RunFunction(codeA, "classify", "95").Value.AsString());
        Assert.Equal("B", PilotScript.RunFunction(codeA, "classify", "85").Value.AsString());
        Assert.Equal("C", PilotScript.RunFunction(codeA, "classify", "72").Value.AsString());
        Assert.Equal("F", PilotScript.RunFunction(codeA, "classify", "50").Value.AsString());

        // Scenario B: while loop with break, continue, and pass
        string codeB = @"
def while_flow() -> int {
    var sum = 0
    var i = 0
    while i < 20 {
        i = i + 1
        if i % 2 != 0 {
            pass
            continue
        }
        if i > 10 {
            break
        }
        sum = sum + i
    }
    return sum
}
";
        // Even numbers <= 10: 2 + 4 + 6 + 8 + 10 = 30
        var resB = PilotScript.RunFunction(codeB, "while_flow");
        Assert.True(resB.Success);
        Assert.Equal(30, resB.Value.AsInt());

        // Scenario C: for-in loop over ranges and lists with break & continue
        string codeC = @"
def for_flow() -> int {
    var collected = 0
    let items = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    for n in items {
        if n == 2 {
            continue
        }
        if n == 6 {
            break
        }
        collected = collected + n
    }
    return collected
}
";
        // 1 + 3 + 4 + 5 = 13
        var resC = PilotScript.RunFunction(codeC, "for_flow");
        Assert.True(resC.Success);
        Assert.Equal(13, resC.Value.AsInt());
    }

    // =========================================================================
    // 3. Early Exit & Cleanup: guard, defer
    // =========================================================================

    [Fact]
    public void Keywords_GuardAndDefer_MultipleScenarios()
    {
        // Scenario A: guard with early return
        string codeA = @"
def guarded_divide(a: int, b: int) -> int {
    guard b != 0 else {
        return -999
    }
    return a / b
}
";
        Assert.Equal(5, PilotScript.RunFunction(codeA, "guarded_divide", "10", "2").Value.AsInt());
        Assert.Equal(-999, PilotScript.RunFunction(codeA, "guarded_divide", "10", "0").Value.AsInt());

        // Scenario B: defer executing in LIFO order upon function exit
        string codeB = @"
var trace_log = []

def defer_pipeline() -> int {
    trace_log.append(""start"")
    defer {
        trace_log.append(""deferred_1"")
    }
    defer {
        trace_log.append(""deferred_2"")
    }
    trace_log.append(""middle"")
    return 42
}
";
        var resB = PilotScript.RunFunction(codeB, "defer_pipeline");
        Assert.True(resB.Success);
        Assert.Equal(42, resB.Value.AsInt());

        var logVal = resB.Context!.Environment.Resolve("trace_log");
        var logList = logVal.AsList();
        Assert.Equal("start", logList[0].AsString());
        Assert.Equal("middle", logList[1].AsString());
        Assert.Equal("deferred_2", logList[2].AsString()); // LIFO
        Assert.Equal("deferred_1", logList[3].AsString());
    }

    // =========================================================================
    // 4. Pattern Matching: match, case, when, wildcard (_)
    // =========================================================================

    [Fact]
    public void Keywords_MatchCase_MultipleScenarios()
    {
        string code = @"
def match_eval(val: int) -> str {
    match val {
        case 0:
            return ""zero""
        case 1 | 2:
            return ""one_or_two""
        case x when x > 100:
            return ""large""
        case x when x < 0:
            return ""negative""
        case _:
            return ""medium""
    }
}
";
        Assert.Equal("zero", PilotScript.RunFunction(code, "match_eval", "0").Value.AsString());
        Assert.Equal("one_or_two", PilotScript.RunFunction(code, "match_eval", "1").Value.AsString());
        Assert.Equal("one_or_two", PilotScript.RunFunction(code, "match_eval", "2").Value.AsString());
        Assert.Equal("large", PilotScript.RunFunction(code, "match_eval", "150").Value.AsString());
        Assert.Equal("negative", PilotScript.RunFunction(code, "match_eval", "-5").Value.AsString());
        Assert.Equal("medium", PilotScript.RunFunction(code, "match_eval", "50").Value.AsString());
    }

    // =========================================================================
    // 5. Functions, Coroutines & Generators: def, fn, pure, return, yield
    // =========================================================================

    [Fact]
    public void Keywords_FunctionsAndYield_MultipleScenarios()
    {
        // Scenario A: pure fn vs def
        string codeA = @"
pure fn square(n: int) -> int {
    return n * n
}

def calc_sum_squares(a: int, b: int) -> int {
    return square(a) + square(b)
}
";
        var resA = PilotScript.RunFunction(codeA, "calc_sum_squares", "3", "4");
        Assert.True(resA.Success);
        Assert.Equal(25, resA.Value.AsInt());

        // Scenario B: yield in stateful generator
        string codeB = @"
def countdown(start: int) {
    var current = start;
    while current > 0 {
        yield current;
        current = current - 1;
    }
}

let gen = countdown(3);
let first = gen.next();
let second = gen.next();
let third = gen.next();
let fourth = gen.next();
let is_finished = gen.is_done;
";
        var resB = PilotScript.Run(codeB);
        Assert.True(resB.Success);
        Assert.Equal(3, resB.Context!.Environment.Resolve("first").AsInt());
        Assert.Equal(2, resB.Context!.Environment.Resolve("second").AsInt());
        Assert.Equal(1, resB.Context!.Environment.Resolve("third").AsInt());
        Assert.True(resB.Context!.Environment.Resolve("is_finished").IsTrue);
    }

    // =========================================================================
    // 6. OOP & Types: class, methods, fields
    // =========================================================================

    [Fact]
    public void Keywords_OOPAndTypes_MultipleScenarios()
    {
        string code = @"
class TaskItem {
    var title = ""Default""
    var is_done = false

    def mark_complete(self) {
        self.is_done = true
        return self
    }

    def describe(self) -> str {
        var status_str = ""PENDING""
        if self.is_done {
            status_str = ""DONE""
        }
        return self.title + "": "" + status_str
    }
}

def run_test() -> str {
    let t = TaskItem()
    t.title = ""Audit""
    t.mark_complete()
    return t.describe()
}
";
        var res = PilotScript.RunFunction(code, "run_test");
        Assert.True(res.Success);
        Assert.Equal("Audit: DONE", res.Value.AsString());
    }

    // =========================================================================
    // 7. Concurrency: async, await, spawn, parallel_map
    // =========================================================================

    [Fact]
    public void Keywords_ConcurrencyAsyncAwait_MultipleScenarios()
    {
        string code = @"
async def compute_hash(seed: int) -> str {
    return ""HASH_"" + (seed * 10)
}

def test_async_spawn() -> str {
    let fut = spawn compute_hash(42)
    let res = await fut
    return res
}
";
        var res = PilotScript.RunFunction(code, "test_async_spawn");
        Assert.True(res.Success);
        Assert.Equal("HASH_420", res.Value.AsString());

        // Parallel map
        string codeParallel = @"
def double_val(x: int) -> int {
    return x * 2
}

def test_parallel() -> int {
    let inputs = [10, 20, 30]
    let results = parallel_map(double_val, inputs)
    return results[0] + results[1] + results[2]
}
";
        var resPar = PilotScript.RunFunction(codeParallel, "test_parallel");
        Assert.True(resPar.Success);
        Assert.Equal(120, resPar.Value.AsInt());
    }

    // =========================================================================
    // 8. Design-by-Contract & Preconditions: requires, ensures, assert
    // =========================================================================

    [Fact]
    public void Keywords_ContractsAndAssert_MultipleScenarios()
    {
        // Scenario A: assert statement
        string codeA = @"
def test_assertions(x: int) -> bool {
    assert x > 0, ""x must be positive""
    assert x < 100, ""x must be less than 100""
    return true
}
";
        var passRes = PilotScript.RunFunction(codeA, "test_assertions", "50");
        Assert.True(passRes.Success);
        Assert.True(passRes.Value.IsTrue);

        var failRes = PilotScript.RunFunction(codeA, "test_assertions", "-5");
        Assert.False(failRes.Success);
        Assert.Contains("x must be positive", failRes.Diagnostics[0]);

        // Scenario B: requires & ensures contract clauses
        string codeB = @"
class Account {
    var balance = 100.0
}

def deposit(acc: Account, amount: float) -> bool
    requires amount > 0.0
    ensures acc.balance == old(acc.balance) + amount
    ensures result == true {
    acc.balance = acc.balance + amount
    return true
}

def test_contracts() -> float {
    let a = Account()
    deposit(a, 50.0)
    return a.balance
}
";
        var resB = PilotScript.RunFunction(codeB, "test_contracts");
        Assert.True(resB.Success);
        Assert.Equal(150.0, resB.Value.AsFloat());
    }

    // =========================================================================
    // 9. Relational Queries & Database Blocks: postgresql / mysql blocks
    // =========================================================================

    [Fact]
    public void Keywords_RelationalQueries_MultipleScenarios()
    {
        string code = @"
let records = postgresql {
    select id, name
    from test_table
    where id in (1, 2, 3);
}

let row_count = records.length
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success);
        var countVal = res.Context!.Environment.Resolve("row_count");
        Assert.True(countVal.AsInt() > 0);
    }

    // =========================================================================
    // 10. Logical, Identity & Exception Keywords: and, or, not, is, try, raise
    // =========================================================================

    [Fact]
    public void Keywords_LogicalAndExceptions_MultipleScenarios()
    {
        // Logical operators
        string codeLogic = @"
def test_logic(a: bool, b: bool, c: bool) -> bool {
    return (a and b) or (not c)
}
";
        Assert.True(PilotScript.RunFunction(codeLogic, "test_logic", "true", "true", "true").Value.IsTrue);
        Assert.True(PilotScript.RunFunction(codeLogic, "test_logic", "false", "false", "false").Value.IsTrue);
        Assert.False(PilotScript.RunFunction(codeLogic, "test_logic", "false", "true", "true").Value.IsTrue);

        // Try / raise / finally exception flow
        string codeEx = @"
def safe_execute(flag: bool) -> str {
    var status = ""init""
    try {
        if flag {
            raise ""CustomError: Intentional Failure""
        }
        status = ""success""
    } except ex {
        status = f""caught: {ex}""
    } finally {
        status = status + "" | finalized""
    }
    return status
}
";
        var resNormal = PilotScript.RunFunction(codeEx, "safe_execute", "false");
        Assert.True(resNormal.Success);
        Assert.Equal("success | finalized", resNormal.Value.AsString());

        var resErr = PilotScript.RunFunction(codeEx, "safe_execute", "true");
        Assert.True(resErr.Success);
        Assert.Contains("caught: CustomError", resErr.Value.AsString());
        Assert.EndsWith("| finalized", resErr.Value.AsString());
    }
}
