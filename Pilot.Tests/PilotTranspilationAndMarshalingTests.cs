using System;
using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Interop;
using Pilot.Runtime.Values;
using Pilot.Transpilation;
using Pilot.Transpilation.Targets;
using Xunit;

namespace Pilot.Tests;

public class PilotTranspilationAndMarshalingTests
{
    private readonly PilotEngine _engine = new();

    #region Compile-Time Conversion: Pilot -> Target Languages

    [Fact]
    public void Test_Transpilation_Pilot_To_Python()
    {
        string pilotCode = @"
            def calculate_total(price: float, quantity: int) -> float {
                let subtotal = price * quantity;
                if subtotal > 100.0 {
                    return subtotal * 0.9;
                } else {
                    return subtotal;
                }
            }

            let total = calculate_total(25.0, 5);
        ";

        var result = _engine.TranspileTo(pilotCode, "python");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("def calculate_total(price: float, quantity: int) -> float:", result.Code);
        Assert.Contains("subtotal = price * quantity", result.Code);
        Assert.Contains("if subtotal > 100.0:", result.Code);
        Assert.Contains("return subtotal * 0.9", result.Code);
        Assert.Contains("else:", result.Code);
    }

    [Fact]
    public void Test_Transpilation_Pilot_To_CSharp()
    {
        string pilotCode = @"
            class Account {
                def get_balance(self) -> float {
                    return 1000.0;
                }
            }

            def process_account() -> void {
                let acc = Account();
                let balance = acc.get_balance();
            }
        ";

        var result = _engine.TranspileTo(pilotCode, "csharp", new TranspilationOptions().WithNamespace("BankingApp"));
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("namespace BankingApp;", result.Code);
        Assert.Contains("public class Account", result.Code);
        Assert.Contains("public static void process_account()", result.Code);
    }

    [Fact]
    public void Test_Transpilation_Pilot_To_Rust()
    {
        string pilotCode = @"
            def add_numbers(a: i64, b: i64) -> i64 {
                let sum: i64 = a + b;
                return sum;
            }
        ";

        var result = _engine.TranspileTo(pilotCode, "rust");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("pub fn add_numbers(a: i64, b: i64) -> i64", result.Code);
        Assert.Contains("let mut sum: i64 = a + b;", result.Code);
    }

    [Fact]
    public void Test_Transpilation_Pilot_To_Java()
    {
        string pilotCode = @"
            class Account {
                let balance: f64 = 1000.0;
                def get_balance(self) -> f64 {
                    return self.balance;
                }
            }

            def add_bonus(amount: f64) -> f64 {
                let bonus: f64 = amount * 0.1;
                return amount + bonus;
            }
        ";

        var result = _engine.TranspileTo(pilotCode, "java", new TranspilationOptions().WithNamespace("com.banking"));
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("package com.banking;", result.Code);
        Assert.Contains("public static class Account", result.Code);
        Assert.Contains("public double get_balance()", result.Code);
        Assert.Contains("public static double add_bonus(double amount)", result.Code);
    }

    [Fact]
    public void Test_Transpilation_Pilot_To_Ruby()
    {
        string pilotCode = @"
            class Account {
                let owner: str = ""Alice"";
                def greet(self) -> str {
                    return f""Hello {self.owner}"";
                }
            }

            let acc = Account(""Bob"");
            println(acc.greet());
        ";

        var result = _engine.TranspileTo(pilotCode, "ruby");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("# frozen_string_literal: true", result.Code);
        Assert.Contains("class Account", result.Code);
        Assert.Contains("attr_accessor :owner", result.Code);
        Assert.Contains("\"Hello #{@owner}\"", result.Code);
        Assert.Contains("Account.new(\"Bob\")", result.Code);
    }

    #endregion

    #region Compile-Time Ingestion: Target Languages -> Pilot

    [Fact]
    public void Test_Ingestion_Python_To_Pilot()
    {
        string pythonCode = @"
def greet(name: str):
    if name == ""admin"":
        return ""Welcome Admin""
    else:
        return ""Hello "" + name
";

        var result = _engine.TranspileFrom(pythonCode, "python");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("def greet(name: str) {", result.Code);
        Assert.Contains("if name == \"admin\" {", result.Code);
        Assert.Contains("return \"Welcome Admin\";", result.Code);
        Assert.Contains("else {", result.Code);
    }

    [Fact]
    public void Test_Ingestion_CSharp_To_Pilot()
    {
        string csharpCode = @"
public class MathUtil
{
    public static int DoubleVal(int x)
    {
        var result = x * 2;
        return result;
    }
}
";

        var result = _engine.TranspileFrom(csharpCode, "csharp");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("class MathUtil", result.Code);
        Assert.Contains("def DoubleVal(x: int)", result.Code);
        Assert.Contains("let result = x * 2;", result.Code);
    }

    [Fact]
    public void Test_Ingestion_Rust_To_Pilot()
    {
        string rustCode = @"
pub fn multiply(a: i64, b: i64) -> i64 {
    let mut res = a * b;
    return res;
}
";

        var result = _engine.TranspileFrom(rustCode, "rust");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("def multiply(a: i64, b: i64) -> i64 {", result.Code);
        Assert.Contains("let res = a * b;", result.Code);
    }

    [Fact]
    public void Test_Ingestion_Java_To_Pilot()
    {
        string javaCode = @"
package com.demo;
import java.util.*;

public class MathHelper {
    public static long classify(int code) {
        final long baseVal = code * 2;
        switch (code) {
            case 1 -> {
                return 10;
            }
            default -> {
                return baseVal;
            }
        }
    }
}
";

        var result = _engine.TranspileFrom(javaCode, "java");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("class MathHelper", result.Code);
        Assert.Contains("def classify(code: int) -> i64", result.Code);
        Assert.Contains("match code", result.Code);
    }

    [Fact]
    public void Test_Ingestion_Ruby_To_Pilot()
    {
        string rubyCode = @"
# frozen_string_literal: true
class Greeter
  attr_accessor :prefix
  def initialize(prefix = ""Hi"")
    @prefix = prefix
  end
  def format_msg(name)
    ""#{@prefix}, #{name}!""
  end
end
";

        var result = _engine.TranspileFrom(rubyCode, "ruby");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Contains("class Greeter", result.Code);
        Assert.Contains("def format_msg(self, name)", result.Code);
        Assert.Contains("return f\"{self.prefix}, {name}!\";", result.Code);
    }

    #endregion

    #region Polyglot Marshaling: Equivalent Types Conversion

    [Fact]
    public void Test_Polyglot_Bridge_Marshals_Primitives()
    {
        var bridge = PolyglotTypeBridge.Default;

        Assert.Equal(42, bridge.ToPilotValue(42).AsInt());
        Assert.Equal(3.14159, bridge.ToPilotValue(3.14159).AsFloat(), 4);
        Assert.True(bridge.ToPilotValue(true).IsTrue);
        Assert.Equal("Pilot Language", bridge.ToPilotValue("Pilot Language").AsString());
        Assert.True(bridge.ToPilotValue(null).IsNull);
    }

    [Fact]
    public void Test_Polyglot_Bridge_Marshals_Collections_And_Maps()
    {
        var bridge = PolyglotTypeBridge.Default;

        // List marshaling
        var rawList = new List<object> { 10, "item", true };
        var pilotList = bridge.ToPilotValue(rawList);
        Assert.Equal(PilotValueKind.List, pilotList.Kind);
        Assert.Equal(3, pilotList.AsList().Count);
        Assert.Equal(10, pilotList.AsList()[0].AsInt());
        Assert.Equal("item", pilotList.AsList()[1].AsString());
        Assert.True(pilotList.AsList()[2].IsTrue);

        // Dictionary marshaling
        var rawDict = new Dictionary<string, object>
        {
            ["key1"] = 100,
            ["key2"] = "value2"
        };
        var pilotMap = bridge.ToPilotValue(rawDict);
        Assert.Equal(PilotValueKind.Map, pilotMap.Kind);
        Assert.Equal(100, pilotMap[PilotValue.FromString("key1")].AsInt());
        Assert.Equal("value2", pilotMap[PilotValue.FromString("key2")].AsString());
    }

    [Fact]
    public void Test_Polyglot_Bridge_Marshals_Poco_Objects()
    {
        var bridge = PolyglotTypeBridge.Default;

        var poco = new TestPerson { Name = "Alice", Age = 28, IsActive = true };
        var pilotObj = bridge.ToPilotValue(poco);

        Assert.Equal(PilotValueKind.Object, pilotObj.Kind);
        Assert.Equal("TestPerson", pilotObj.AsObject().TypeName);
        Assert.Equal("Alice", pilotObj.GetMember("Name").AsString());
        Assert.Equal(28, pilotObj.GetMember("Age").AsInt());
        Assert.True(pilotObj.GetMember("IsActive").IsTrue);
    }

    [Fact]
    public void Test_Polyglot_Provider_Returns_Equivalent_Types()
    {
        var registry = new PolyglotRegistry();

        // Python provider invokes return typed floats
        var pyProvider = registry.Get("python");
        var pyVal = pyProvider.Invoke("sin", new[] { PilotValue.FromFloat(0.0) }, null!);
        Assert.Equal(PilotValueKind.Float, pyVal.Kind);
        Assert.Equal(0.0, pyVal.AsFloat());

        // C# provider invokes return typed strings and dates
        var csProvider = registry.Get("csharp");
        var csVal = csProvider.Invoke("NewGuid", Array.Empty<PilotValue>(), null!);
        Assert.Equal(PilotValueKind.String, csVal.Kind);
        Assert.True(Guid.TryParse(csVal.AsString(), out _));
    }

    private class TestPerson
    {
        public string Name { get; set; } = "";
        public int Age { get; set; }
        public bool IsActive { get; set; }
    }

    #endregion
}
