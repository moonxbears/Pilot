#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotContractDelegationAndStealingTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Contract_Forwarding_IList_To_Underlying_List()
    {
        string code = @"
            class ItemContainer(IList) {
                let _items = [];
                IList.Contracts = _items;
            }

            let container = ItemContainer();
            container.append(""apple"");
            container.append(""banana"");

            let cnt = container.count();
            let first = container.get(0);
            let second = container.get(1);
            let hasApple = container.contains(""apple"");
            let hasOrange = container.contains(""orange"");
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var cnt = result.Context.Environment.CurrentScope.Resolve("cnt");
        var first = result.Context.Environment.CurrentScope.Resolve("first");
        var second = result.Context.Environment.CurrentScope.Resolve("second");
        var hasApple = result.Context.Environment.CurrentScope.Resolve("hasApple");
        var hasOrange = result.Context.Environment.CurrentScope.Resolve("hasOrange");

        Assert.Equal(2, cnt.AsInt());
        Assert.Equal("apple", first.AsString());
        Assert.Equal("banana", second.AsString());
        Assert.True(hasApple.IsTrue);
        Assert.False(hasOrange.IsTrue);
    }

    [Fact]
    public void Test_Contract_Forwarding_With_Custom_Method_Override()
    {
        string code = @"
            class CustomList(IList) {
                let _items = [];
                IList.Contracts = _items;

                // Custom override for count to multiply by 100
                def count(self) {
                    return self._items.length * 100;
                }
            }

            let list = CustomList();
            list.append(10);
            list.append(20);

            // count should use the custom override
            let customCount = list.count();
            // get should use the default contract forwarder
            let first = list.get(0);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var customCount = result.Context.Environment.CurrentScope.Resolve("customCount");
        var first = result.Context.Environment.CurrentScope.Resolve("first");

        Assert.Equal(200, customCount.AsInt());
        Assert.Equal(10, first.AsInt());
    }

    [Fact]
    public void Test_Contract_Forwarding_Generic_Syntax()
    {
        string code = @"
            class TypedCollection(IList) {
                let _data = [];
                IList<PilotValue>.Contracts = _data;
            }

            let tc = TypedCollection();
            tc.append(42);
            tc.append(99);

            let cnt = tc.count();
            let item = tc.get(1);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var cnt = result.Context.Environment.CurrentScope.Resolve("cnt");
        var item = result.Context.Environment.CurrentScope.Resolve("item");

        Assert.Equal(2, cnt.AsInt());
        Assert.Equal(99, item.AsInt());
    }

    [Fact]
    public void Test_Contract_Forwarding_IEquatable()
    {
        string code = @"
            class Account(IEquatable) {
                def __init__(self, accNumber, balance) {
                    self.accNumber = accNumber;
                    self.balance = balance;
                    IEquatable.Contracts = accNumber;
                }
            }

            let acc1 = Account(""ACC-001"", 500);
            let acc2 = Account(""ACC-001"", 9999);
            let acc3 = Account(""ACC-002"", 500);

            let eq1_2 = acc1.equals(acc2);
            let eq1_3 = acc1.equals(acc3);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var eq1_2 = result.Context.Environment.CurrentScope.Resolve("eq1_2");
        var eq1_3 = result.Context.Environment.CurrentScope.Resolve("eq1_3");

        Assert.True(eq1_2.IsTrue);
        Assert.False(eq1_3.IsTrue);
    }

    [Fact]
    public void Test_Method_Stealing_From_Non_Inherited_Class()
    {
        string code = @"
            class LoggerUtility {
                def log_info(self, message) {
                    return ""[INFO] "" + message;
                }
                def log_warn(self, message) {
                    return ""[WARN] "" + message;
                }
            }

            class OrderProcessor {
                steal LoggerUtility;

                def process_order(self, orderId) {
                    return self.log_info(""Processed order "" + str(orderId));
                }
            }

            let processor = OrderProcessor();
            let msg1 = processor.process_order(101);
            let msg2 = processor.log_warn(""Inventory low"");
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var msg1 = result.Context.Environment.CurrentScope.Resolve("msg1");
        var msg2 = result.Context.Environment.CurrentScope.Resolve("msg2");

        Assert.Equal("[INFO] Processed order 101", msg1.AsString());
        Assert.Equal("[WARN] Inventory low", msg2.AsString());
    }

    [Fact]
    public void Test_Method_Stealing_With_Custom_Override()
    {
        string code = @"
            class BaseCalculator {
                def add(self, a, b) {
                    return a + b;
                }
                def multiply(self, a, b) {
                    return a * b;
                }
            }

            class AdvancedCalculator {
                steal BaseCalculator;

                // Override add with custom scaling
                def add(self, a, b) {
                    return (a + b) * 10;
                }
            }

            let calc = AdvancedCalculator();
            let addResult = calc.add(2, 3);
            let multResult = calc.multiply(4, 5);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var addResult = result.Context.Environment.CurrentScope.Resolve("addResult");
        var multResult = result.Context.Environment.CurrentScope.Resolve("multResult");

        Assert.Equal(50, addResult.AsInt());
        Assert.Equal(20, multResult.AsInt());
    }

    [Fact]
    public void Test_Method_Stealing_Specific_Methods()
    {
        string code = @"
            class MathToolkit {
                def square(self, x) {
                    return x * x;
                }
                def cube(self, x) {
                    return x * x * x;
                }
            }

            class SelectiveConsumer {
                steal from MathToolkit [square];
            }

            let sc = SelectiveConsumer();
            let sq = sc.square(6);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var sq = result.Context.Environment.CurrentScope.Resolve("sq");
        Assert.Equal(36, sq.AsInt());
    }
}
