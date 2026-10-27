#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotContextContractsPatternsTests
{
    private readonly PilotEngine _engine = new();

    #region 1. Contextual Parameters (Native Dependency Injection)

    [Fact]
    public void Test_Contextual_Parameters_Implicit_Downstream_Propagation()
    {
        string code = @"
            class Database {
                let connectionString = """";
                let executed = [];

                def execute(self, query, param) {
                    self.executed.append(query + "":"" + param);
                    return ""ok"";
                }
            }

            def save_user(user, context db: Database) {
                db.execute(""INSERT"", user);
                return ""saved"";
            }

            def process_registration(payload, context db: Database) {
                let user = payload[""username""];
                # db is automatically implicitly passed to save_user
                return save_user(user);
            }

            var result = """";
            let dbInstance = Database();
            dbInstance.connectionString = ""postgresql://localhost:5432"";

            with context db = dbInstance {
                result = process_registration({ ""username"": ""Isaac"" });
            }

            let recordedQueries = dbInstance.executed;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var resVal = result.Context.Environment.CurrentScope.Resolve("result");
        var queries = result.Context.Environment.CurrentScope.Resolve("recordedQueries");

        Assert.Equal("saved", resVal.AsString());
        Assert.Equal("INSERT:Isaac", queries.AsList()[0].AsString());
    }

    [Fact]
    public void Test_Contextual_Parameters_Explicit_Override()
    {
        string code = @"
            class Service {
                let name = ""default"";
            }

            def get_service_name(context svc: Service) {
                return svc.name;
            }

            let mainSvc = Service();
            mainSvc.name = ""main"";

            let customSvc = Service();
            customSvc.name = ""custom"";

            var resImplicit = """";
            var resExplicit = """";

            with context svc = mainSvc {
                resImplicit = get_service_name();
                resExplicit = get_service_name(customSvc);
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        Assert.Equal("main", result.Context.Environment.CurrentScope.Resolve("resImplicit").AsString());
        Assert.Equal("custom", result.Context.Environment.CurrentScope.Resolve("resExplicit").AsString());
    }

    [Fact]
    public void Test_Contextual_Parameters_Missing_Throws_Error()
    {
        string code = @"
            def needs_context(context requiredConfig) {
                return requiredConfig;
            }

            let val = needs_context();
        ";

        var result = _engine.Execute(code);
        Assert.False(result.Success);
        Assert.Contains("ContextError", string.Join(" ", result.Diagnostics));
    }

    #endregion

    #region 2. Exhaustive Pattern Matching Assignments

    [Fact]
    public void Test_Exhaustive_Pattern_Matching_Assignment_Record()
    {
        string code = @"
            def fetch_user_record() {
                return {
                    ""id"": 101,
                    ""metadata"": {
                        ""role"": ""Admin"",
                        ""department"": ""Engineering""
                    }
                };
            }

            # Direct pattern matching assignment without let
            { ""id"": user_id, ""metadata"": { ""role"": user_role } } = fetch_user_record();
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var userId = result.Context.Environment.CurrentScope.Resolve("user_id");
        var userRole = result.Context.Environment.CurrentScope.Resolve("user_role");

        Assert.Equal(101, userId.AsInt());
        Assert.Equal("Admin", userRole.AsString());
    }

    [Fact]
    public void Test_Exhaustive_Pattern_Matching_Assignment_Spread_List()
    {
        string code = @"
            let numbers = [10, 20, 30, 40, 50];
            [first, ..middle, last] = numbers;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var first = result.Context.Environment.CurrentScope.Resolve("first");
        var middle = result.Context.Environment.CurrentScope.Resolve("middle");
        var last = result.Context.Environment.CurrentScope.Resolve("last");

        Assert.Equal(10, first.AsInt());
        Assert.Equal(50, last.AsInt());
        Assert.Equal(3, middle.AsList().Count);
        Assert.Equal(20, middle.AsList()[0].AsInt());
        Assert.Equal(40, middle.AsList()[2].AsInt());
    }

    [Fact]
    public void Test_Exhaustive_Pattern_Matching_Missing_Key_Throws_Error()
    {
        string code = @"
            let payload = { ""name"": ""Alice"" };
            # Fails because 'id' is required by the pattern
            { ""id"": userId, ""name"": userName } = payload;
        ";

        var result = _engine.Execute(code);
        Assert.False(result.Success);
        Assert.Contains("MatchError", string.Join(" ", result.Diagnostics));
    }

    [Fact]
    public void Test_Exhaustive_Pattern_Matching_Length_Mismatch_Throws_Error()
    {
        string code = @"
            let coords = [1, 2, 3];
            # Fails because pattern expects exactly 2 elements
            [x, y] = coords;
        ";

        var result = _engine.Execute(code);
        Assert.False(result.Success);
        Assert.Contains("MatchError", string.Join(" ", result.Diagnostics));
    }

    #endregion

    #region 3. Design-by-Contract (Signature Validation)

    [Fact]
    public void Test_Design_By_Contract_Preconditions_And_Postconditions_Success()
    {
        string code = @"
            class Account {
                let balance = 100.0;

                def deduct(self, amt) {
                    self.balance = self.balance - amt;
                }

                def add(self, amt) {
                    self.balance = self.balance + amt;
                }
            }

            def transfer_funds(amount: float, source: Account, dest: Account)
                requires amount > 0.0
                requires source.balance >= amount
                ensures source.balance == old(source.balance) - amount
                ensures dest.balance == old(dest.balance) + amount
                ensures result == true {

                source.deduct(amount);
                dest.add(amount);
                return true;
            }

            let acc1 = Account();
            acc1.balance = 500.0;

            let acc2 = Account();
            acc2.balance = 100.0;

            let transferSuccess = transfer_funds(200.0, acc1, acc2);
            let finalBalance1 = acc1.balance;
            let finalBalance2 = acc2.balance;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var transferSuccess = result.Context.Environment.CurrentScope.Resolve("transferSuccess");
        var b1 = result.Context.Environment.CurrentScope.Resolve("finalBalance1");
        var b2 = result.Context.Environment.CurrentScope.Resolve("finalBalance2");

        Assert.True(transferSuccess.IsTrue);
        Assert.Equal(300.0, b1.AsFloat());
        Assert.Equal(300.0, b2.AsFloat());
    }

    [Fact]
    public void Test_Design_By_Contract_Precondition_Violation_Throws_Error()
    {
        string code = @"
            class Account {
                let balance = 50.0;
            }

            def withdraw(amount: float, acc: Account)
                requires amount > 0.0
                requires acc.balance >= amount {
                acc.balance = acc.balance - amount;
                return acc.balance;
            }

            let a = Account();
            # Violates precondition: acc.balance >= 100.0 is false (50.0 < 100.0)
            let res = withdraw(100.0, a);
        ";

        var result = _engine.Execute(code);
        Assert.False(result.Success);
        Assert.Contains("ContractError", string.Join(" ", result.Diagnostics));
        Assert.Contains("Precondition failed", string.Join(" ", result.Diagnostics));
    }

    [Fact]
    public void Test_Design_By_Contract_Postcondition_Violation_Throws_Error()
    {
        string code = @"
            def buggy_increment(x: int)
                requires x >= 0
                ensures result == old(x) + 1 {
                # Bug: adds 2 instead of 1
                return x + 2;
            }

            let res = buggy_increment(5);
        ";

        var result = _engine.Execute(code);
        Assert.False(result.Success);
        Assert.Contains("ContractError", string.Join(" ", result.Diagnostics));
        Assert.Contains("Postcondition failed", string.Join(" ", result.Diagnostics));
    }

    #endregion
}
