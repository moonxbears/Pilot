using Pilot.Runtime.Values;

namespace Pilot.Runtime.Environment;

/// <summary>
/// Represents a variable binding within a <see cref="PilotScope"/>.
/// </summary>
public sealed class PilotBinding
{
    public string Name { get; }
    public PilotValue Value { get; set; }
    public bool IsConst { get; }
    public string? TypeName { get; }

    public PilotBinding(string name, PilotValue value, bool isConst = false, string? typeName = null)
    {
        Name = name;
        Value = value;
        IsConst = isConst;
        TypeName = typeName;
    }
}
