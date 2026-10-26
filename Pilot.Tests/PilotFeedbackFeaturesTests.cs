#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotFeedbackFeaturesTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Algebraic_Class_Composition_Union()
    {
        string code = @"
            class BaseUser {
                let name = ""Alice"";
                def get_role(self) {
                    return ""base"";
                }
            }

            class AdminPrivileges {
                let permissions = [""read"", ""write""];
                def grant_access(self) {
                    return ""granted"";
                }
            }

            # Algebraic union combines properties and methods
            class Admin = BaseUser | AdminPrivileges;

            let admin = Admin();
            let role = admin.get_role();
            let access = admin.grant_access();
            let adminName = admin.name;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var role = result.Context.Environment.CurrentScope.Resolve("role");
        var access = result.Context.Environment.CurrentScope.Resolve("access");
        var adminName = result.Context.Environment.CurrentScope.Resolve("adminName");

        Assert.Equal("base", role.AsString());
        Assert.Equal("granted", access.AsString());
        Assert.Equal("Alice", adminName.AsString());
    }

    [Fact]
    public void Test_Algebraic_Class_Composition_Subtraction()
    {
        string code = @"
            class BaseUser {
                let name = ""Bob"";
                def get_role(self) {
                    return ""user"";
                }
                def bill(self) {
                    return ""$10"";
                }
            }

            class BillingModule {
                def bill(self) {
                    return ""$0"";
                }
            }

            # Subtraction removes methods/fields present in BillingModule
            class FreeUser = BaseUser - BillingModule;

            let free = FreeUser();
            let role = free.get_role();
            let freeName = free.name;
            let billFunc = free.bill;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var role = result.Context.Environment.CurrentScope.Resolve("role");
        var freeName = result.Context.Environment.CurrentScope.Resolve("freeName");
        var billFunc = result.Context.Environment.CurrentScope.Resolve("billFunc");

        Assert.Equal("user", role.AsString());
        Assert.Equal("Bob", freeName.AsString());
        Assert.True(billFunc.IsUndefined);
    }

    [Fact]
    public void Test_Algebraic_Class_Composition_Intersection()
    {
        string code = @"
            class ServiceA {
                def ping(self) {
                    return ""pong"";
                }
                def alpha(self) {
                    return 1;
                }
            }

            class ServiceB {
                def ping(self) {
                    return ""pong"";
                }
                def beta(self) {
                    return 2;
                }
            }

            # Intersection only keeps overlapping methods/fields
            class CommonInterface = ServiceA & ServiceB;

            let common = CommonInterface();
            let pingRes = common.ping();
            let alphaFunc = common.alpha;
            let betaFunc = common.beta;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var pingRes = result.Context.Environment.CurrentScope.Resolve("pingRes");
        var alphaFunc = result.Context.Environment.CurrentScope.Resolve("alphaFunc");
        var betaFunc = result.Context.Environment.CurrentScope.Resolve("betaFunc");

        Assert.Equal("pong", pingRes.AsString());
        Assert.True(alphaFunc.IsUndefined);
        Assert.True(betaFunc.IsUndefined);
    }

    [Fact]
    public void Test_Typestate_Classes_Morphing()
    {
        string code = @"
            class Connection {
                state Disconnected {
                    def connect(self) {
                        return self.transition_to(""Connected"");
                    }
                }

                state Connected {
                    def send(self, payload) {
                        return ""sent:"" + payload;
                    }
                    def close(self) {
                        return self.transition_to(""Disconnected"");
                    }
                }
            }

            let conn = Connection();
            # Calling connect() transitions to Connected state
            let active = conn.connect();
            let sendResult = active.send(""pilot_payload"");
            active.close();
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var sendResult = result.Context.Environment.CurrentScope.Resolve("sendResult");
        Assert.Equal("sent:pilot_payload", sendResult.AsString());

        // Calling send on Disconnected state must fail with invalid typestate exception
        string invalidCall = @"
            class Connection {
                state Disconnected {
                    def connect(self) {
                        return self.transition_to(""Connected"");
                    }
                }

                state Connected {
                    def send(self, payload) {
                        return ""sent:"" + payload;
                    }
                }
            }

            let c = Connection();
            c.send(""fail_payload"");
        ";
        var invalidResult = _engine.Execute(invalidCall);
        Assert.False(invalidResult.Success);
        Assert.Contains("does not exist on Connection[Disconnected]", string.Join(" ", invalidResult.Diagnostics));
    }

    [Fact]
    public void Test_Structural_Projection()
    {
        string code = @"
            class Employee {
                let name = ""Isaac"";
                let department = ""Engineering"";
                let salary = 120000;
            }

            let employee = Employee();
            # Project specific fields into an anonymous dictionary/map
            let user_dto = employee -> {name, department};

            let dto_name = user_dto[""name""];
            let dto_dept = user_dto[""department""];
            let dto_salary = user_dto[""salary""];
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var dtoName = result.Context.Environment.CurrentScope.Resolve("dto_name");
        var dtoDept = result.Context.Environment.CurrentScope.Resolve("dto_dept");
        var dtoSalary = result.Context.Environment.CurrentScope.Resolve("dto_salary");

        Assert.Equal("Isaac", dtoName.AsString());
        Assert.Equal("Engineering", dtoDept.AsString());
        Assert.True(dtoSalary.IsUndefined);
    }

    [Fact]
    public void Test_Lexical_Extensions_Scoped_Monkey_Patching()
    {
        string code = @"
            extension DebugHelpers on str {
                def to_hex(self) {
                    return self.to_upper();
                }
            }

            let inside_result = """";

            with extension DebugHelpers {
                inside_result = ""hello"".to_hex();
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var insideResult = result.Context.Environment.CurrentScope.Resolve("inside_result");
        Assert.Equal("HELLO", insideResult.AsString());

        // Outside the lexical block, method does not exist
        string outsideCall = @"
            extension DebugHelpers on str {
                def to_hex(self) {
                    return self.to_upper();
                }
            }

            let res = ""hello"".to_hex();
        ";
        var outsideResult = _engine.Execute(outsideCall);
        Assert.False(outsideResult.Success);
    }

    [Fact]
    public void Test_CompileTime_Generative_Classes()
    {
        string code = @"
            class DatabaseModel {
                macro create_crud_methods(ast) {
                    for field in ast.fields {
                        ast.add_method(f""get_{field.name}"", f""return self.{field.name}"");
                    }
                }
            }

            class User(DatabaseModel) {
                name: str
                email: str
            }

            let user = User();
            user.name = ""Isaac"";
            user.email = ""isaac@pilot.lang"";

            let nameVal = user.get_name();
            let emailVal = user.get_email();
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var nameVal = result.Context.Environment.CurrentScope.Resolve("nameVal");
        var emailVal = result.Context.Environment.CurrentScope.Resolve("emailVal");

        Assert.Equal("Isaac", nameVal.AsString());
        Assert.Equal("isaac@pilot.lang", emailVal.AsString());
    }
}
