using System;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Interop;

/// <summary>
/// Contract for bidirectional type marshaling between foreign language values and Pilot values.
/// </summary>
public interface IPolyglotTypeConverter
{
    /// <summary>
    /// Converts a foreign runtime object or primitive into its equivalent <see cref="PilotValue"/>.
    /// </summary>
    PilotValue ToPilotValue(object? foreignObject, string? sourceLanguage = null);

    /// <summary>
    /// Converts a <see cref="PilotValue"/> into a foreign/host object of the requested target type.
    /// </summary>
    object? ToForeignValue(PilotValue value, Type? targetType = null);
}
