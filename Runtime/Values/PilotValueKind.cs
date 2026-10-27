namespace Pilot.Runtime.Values;

/// <summary>
/// Specifies the runtime data type of a <see cref="PilotValue"/>.
/// </summary>
public enum PilotValueKind
{
    Null,
    Undefined,
    Bool,
    Int,
    UInt,
    Float,
    Decimal,
    Char,
    String,
    List,
    Tuple,
    Map,
    Set,
    Range,
    Matrix,
    Function,
    Object,
    Generator,
    Lazy,
    Future,
    Foreign,
    MethodGroup,
    NativeArray,
    NativeMap
}
