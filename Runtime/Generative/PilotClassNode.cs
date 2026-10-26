using System;
using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Generative;

/// <summary>
/// Represents a field in a class AST during compile-time generative expansion.
/// </summary>
public sealed class PilotFieldNode
{
    public string Name { get; set; }
    public string TypeName { get; set; }
    public PilotValue DefaultValue { get; set; }

    public PilotFieldNode(string name, string typeName = "any", PilotValue defaultValue = default)
    {
        Name = name;
        TypeName = typeName;
        DefaultValue = defaultValue.IsUndefined ? PilotValue.Null : defaultValue;
    }

    public PilotObject ToPilotObject()
    {
        var obj = new PilotObject();
        obj.SetMember("name", PilotValue.FromString(Name));
        obj.SetMember("type", PilotValue.FromString(TypeName));
        obj.SetMember("default_value", DefaultValue);
        return obj;
    }
}

/// <summary>
/// AST representation of a class passed to @generate compile-time macros.
/// Allows introspecting and synthesizing new methods or fields dynamically.
/// </summary>
public sealed class PilotClassNode
{
    public PilotClassDefinition TargetClass { get; }
    public string Name => TargetClass.Name;
    public List<PilotFieldNode> Fields { get; } = new();

    public PilotClassNode(PilotClassDefinition targetClass)
    {
        TargetClass = targetClass;

        foreach (var kvp in targetClass.StaticFields)
        {
            Fields.Add(new PilotFieldNode(kvp.Key, "any", kvp.Value));
        }
    }

    public void AddField(string name, string typeName = "any", PilotValue defaultValue = default)
    {
        Fields.Add(new PilotFieldNode(name, typeName, defaultValue));
        TargetClass.StaticFields[name] = defaultValue.IsUndefined ? PilotValue.Null : defaultValue;
    }

    public void AddMethod(string name, PilotFunction function)
    {
        TargetClass.Methods[name] = function;
    }

    public void AddGetterMethod(string name, string fieldName)
    {
        var fn = PilotFunction.CreateNative(name, (ctx, args) =>
        {
            if (args.Count > 0 && args[0].Kind == PilotValueKind.Object)
            {
                var selfObj = args[0].AsObject();
                return selfObj.GetMember(fieldName);
            }
            return PilotValue.Undefined;
        }, "self");

        TargetClass.Methods[name] = fn;
    }

    public PilotObject ToPilotObject(PilotExecutionContext context)
    {
        var obj = new PilotObject();
        obj.SetMember("name", PilotValue.FromString(Name));

        var fieldObjs = new List<PilotValue>(Fields.Count);
        foreach (var f in Fields)
        {
            fieldObjs.Add(PilotValue.FromObject(f.ToPilotObject()));
        }
        obj.SetMember("fields", PilotValue.FromList(fieldObjs));

        // add_method(name, codeOrPattern)
        obj.SetMember("add_method", PilotValue.FromFunction(PilotFunction.CreateNative("add_method", (ctx, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            string methodName = args[0].ToString();
            string bodyCode = args[1].ToString();

            // Check if body is "return self.<field>" or expression
            if (bodyCode.StartsWith("return self.", StringComparison.OrdinalIgnoreCase))
            {
                string targetField = bodyCode.Substring("return self.".Length).Trim(';', ' ');
                AddGetterMethod(methodName, targetField);
            }
            else
            {
                // General synthesized method returning expression or field
                AddGetterMethod(methodName, methodName.StartsWith("get_") ? methodName.Substring(4) : methodName);
            }
            return PilotValue.Null;
        }, "name", "body")));

        // add_field(name, defaultValue)
        obj.SetMember("add_field", PilotValue.FromFunction(PilotFunction.CreateNative("add_field", (ctx, args) =>
        {
            if (args.Count > 0)
            {
                string fName = args[0].ToString();
                var defVal = args.Count > 1 ? args[1] : PilotValue.Null;
                AddField(fName, "any", defVal);
            }
            return PilotValue.Null;
        }, "name", "default_value")));

        return obj;
    }
}
