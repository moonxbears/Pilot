using System;
using System.Runtime.InteropServices;

namespace Pilot.Runtime.Bytecode;

/// <summary>
/// A single bytecode instruction executed by the PilVM execution engine.
/// Structured for minimal footprint, CPU cache line alignment, and zero decoding overhead.
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 4)]
public readonly struct PilotInstruction : IEquatable<PilotInstruction>
{
    /// <summary>The opcode operation to execute.</summary>
    public readonly PilotOpcode Opcode;

    /// <summary>Primary operand (slot index, constant pool index, jump target, or arg count).</summary>
    public readonly int Operand;

    /// <summary>Secondary operand (optional argument count or flag).</summary>
    public readonly int Extra;

    /// <summary>Source line number for diagnostics and stack traces.</summary>
    public readonly int Line;

    public PilotInstruction(PilotOpcode opcode, int operand = 0, int extra = 0, int line = 0)
    {
        Opcode = opcode;
        Operand = operand;
        Extra = extra;
        Line = line;
    }

    public bool Equals(PilotInstruction other) =>
        Opcode == other.Opcode && Operand == other.Operand && Extra == other.Extra && Line == other.Line;

    public override bool Equals(object? obj) => obj is PilotInstruction other && Equals(other);

    public override int GetHashCode() => HashCode.Combine((byte)Opcode, Operand, Extra, Line);

    public static bool operator ==(PilotInstruction left, PilotInstruction right) => left.Equals(right);
    public static bool operator !=(PilotInstruction left, PilotInstruction right) => !left.Equals(right);

    public override string ToString() => Extra != 0
        ? $"{Opcode} {Operand}, {Extra} (line {Line})"
        : $"{Opcode} {Operand} (line {Line})";
}
