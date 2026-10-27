using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Bytecode;

/// <summary>
/// Encapsulates a compiled sequence of bytecode instructions, constant pool,
/// variable bindings, local slot layout, and nested closure chunks.
/// </summary>
public sealed class PilotBytecodeChunk
{
    public string Name { get; set; } = "<main>";
    public List<PilotInstruction> Instructions { get; } = new();
    public List<PilotValue> Constants { get; } = new();
    public List<string> IdentifierPool { get; } = new();
    public List<string> ParameterNames { get; } = new();
    public List<PilotBytecodeChunk> NestedChunks { get; } = new();
    public Dictionary<string, int> LocalSlotMap { get; } = new(StringComparer.Ordinal);
    public int LocalCount { get; set; } = 0;
    public List<IParseTree> AstFallbackNodes { get; } = new();

    public int InstructionCount => Instructions.Count;

    public PilotBytecodeChunk(string name = "<main>")
    {
        Name = name;
    }

    /// <summary>
    /// Emits a single instruction into the chunk.
    /// </summary>
    public void Emit(PilotOpcode opcode, int operand = 0, int extra = 0, int line = 0)
    {
        Instructions.Add(new PilotInstruction(opcode, operand, extra, line));
    }

    /// <summary>
    /// Emits a jump instruction with a dummy target (0), returning the index to be backpatched later.
    /// </summary>
    public int EmitJump(PilotOpcode opcode, int extra = 0, int line = 0)
    {
        int index = Instructions.Count;
        Instructions.Add(new PilotInstruction(opcode, 0, extra, line));
        return index;
    }

    /// <summary>
    /// Backpatches a jump instruction at <paramref name="jumpIndex"/> to target the specified or current instruction index.
    /// </summary>
    public void PatchJump(int jumpIndex, int targetIndex = -1)
    {
        if (targetIndex < 0)
        {
            targetIndex = Instructions.Count;
        }

        var old = Instructions[jumpIndex];
        Instructions[jumpIndex] = new PilotInstruction(old.Opcode, targetIndex, old.Extra, old.Line);
    }

    /// <summary>
    /// Adds a value to the constant pool, returning its index. Reuses identical constants when possible.
    /// </summary>
    public int AddConstant(PilotValue value)
    {
        for (int i = 0; i < Constants.Count; i++)
        {
            if (Constants[i].Equals(value))
            {
                return i;
            }
        }
        Constants.Add(value);
        return Constants.Count - 1;
    }

    /// <summary>
    /// Adds an identifier to the pool, returning its index.
    /// </summary>
    public int AddIdentifier(string identifier)
    {
        for (int i = 0; i < IdentifierPool.Count; i++)
        {
            if (string.Equals(IdentifierPool[i], identifier, StringComparison.Ordinal))
            {
                return i;
            }
        }
        IdentifierPool.Add(identifier);
        return IdentifierPool.Count - 1;
    }

    /// <summary>
    /// Adds a nested closure bytecode chunk, returning its index.
    /// </summary>
    public int AddNestedChunk(PilotBytecodeChunk chunk)
    {
        NestedChunks.Add(chunk);
        return NestedChunks.Count - 1;
    }

    /// <summary>
    /// Registers an AST fallback parse tree node, returning its index.
    /// </summary>
    public int AddAstFallback(IParseTree node)
    {
        AstFallbackNodes.Add(node);
        return AstFallbackNodes.Count - 1;
    }

    /// <summary>
    /// Resolves or allocates a local variable slot index.
    /// </summary>
    public int GetOrAllocateLocalSlot(string name)
    {
        if (LocalSlotMap.TryGetValue(name, out int slot))
        {
            return slot;
        }
        slot = LocalCount++;
        LocalSlotMap[name] = slot;
        return slot;
    }

    /// <summary>
    /// Tries to get the local slot for an existing variable.
    /// </summary>
    public bool TryGetLocalSlot(string name, out int slot)
    {
        return LocalSlotMap.TryGetValue(name, out slot);
    }

    #region Serialization & Deserialization

    public void Write(BinaryWriter writer)
    {
        writer.Write(Name ?? "<main>");
        writer.Write(LocalCount);

        // Parameters
        writer.Write(ParameterNames.Count);
        foreach (var p in ParameterNames)
        {
            writer.Write(p);
        }

        // LocalSlotMap
        writer.Write(LocalSlotMap.Count);
        foreach (var kvp in LocalSlotMap)
        {
            writer.Write(kvp.Key);
            writer.Write(kvp.Value);
        }

        // Identifier pool
        writer.Write(IdentifierPool.Count);
        foreach (var id in IdentifierPool)
        {
            writer.Write(id);
        }

        // Constant pool
        writer.Write(Constants.Count);
        foreach (var c in Constants)
        {
            WriteValue(writer, c);
        }

        // Nested chunks
        writer.Write(NestedChunks.Count);
        foreach (var nc in NestedChunks)
        {
            nc.Write(writer);
        }

        // Instructions
        writer.Write(Instructions.Count);
        foreach (var inst in Instructions)
        {
            writer.Write((byte)inst.Opcode);
            writer.Write(inst.Operand);
            writer.Write(inst.Extra);
            writer.Write(inst.Line);
        }
    }

    public static PilotBytecodeChunk Read(BinaryReader reader)
    {
        string name = reader.ReadString();
        var chunk = new PilotBytecodeChunk(name)
        {
            LocalCount = reader.ReadInt32()
        };

        // Parameters
        int paramCount = reader.ReadInt32();
        for (int i = 0; i < paramCount; i++)
        {
            chunk.ParameterNames.Add(reader.ReadString());
        }

        // LocalSlotMap
        int slotMapCount = reader.ReadInt32();
        for (int i = 0; i < slotMapCount; i++)
        {
            string key = reader.ReadString();
            int val = reader.ReadInt32();
            chunk.LocalSlotMap[key] = val;
        }

        // Identifier pool
        int idCount = reader.ReadInt32();
        for (int i = 0; i < idCount; i++)
        {
            chunk.IdentifierPool.Add(reader.ReadString());
        }

        // Constant pool
        int constCount = reader.ReadInt32();
        for (int i = 0; i < constCount; i++)
        {
            chunk.Constants.Add(ReadValue(reader));
        }

        // Nested chunks
        int nestedCount = reader.ReadInt32();
        for (int i = 0; i < nestedCount; i++)
        {
            chunk.NestedChunks.Add(Read(reader));
        }

        // Instructions
        int instCount = reader.ReadInt32();
        for (int i = 0; i < instCount; i++)
        {
            var op = (PilotOpcode)reader.ReadByte();
            int operand = reader.ReadInt32();
            int extra = reader.ReadInt32();
            int line = reader.ReadInt32();
            chunk.Instructions.Add(new PilotInstruction(op, operand, extra, line));
        }

        return chunk;
    }

    private static void WriteValue(BinaryWriter writer, PilotValue value)
    {
        writer.Write((byte)value.Kind);
        switch (value.Kind)
        {
            case PilotValueKind.Null:
            case PilotValueKind.Undefined:
                break;
            case PilotValueKind.Bool:
                writer.Write(value.IsTrue);
                break;
            case PilotValueKind.Int:
                writer.Write(value.AsInt());
                break;
            case PilotValueKind.UInt:
                writer.Write((ulong)value.AsInt());
                break;
            case PilotValueKind.Float:
                writer.Write(value.AsFloat());
                break;
            case PilotValueKind.Decimal:
                writer.Write(value.AsDecimal());
                break;
            case PilotValueKind.Char:
                writer.Write(value.AsChar());
                break;
            case PilotValueKind.String:
                writer.Write(value.AsString());
                break;
            case PilotValueKind.List:
            {
                var list = value.AsList();
                writer.Write(list.Count);
                foreach (var item in list) WriteValue(writer, item);
                break;
            }
            case PilotValueKind.Tuple:
            {
                var tuple = value.AsTuple();
                writer.Write(tuple.Count);
                foreach (var item in tuple) WriteValue(writer, item);
                break;
            }
            case PilotValueKind.Range:
            {
                var range = value.AsRange();
                writer.Write(range.Start);
                writer.Write(range.End);
                writer.Write(range.Step);
                writer.Write(range.Inclusive);
                break;
            }
            default:
                // Non-serializable objects serialize as Null placeholder in constant pool
                break;
        }
    }

    private static PilotValue ReadValue(BinaryReader reader)
    {
        var kind = (PilotValueKind)reader.ReadByte();
        return kind switch
        {
            PilotValueKind.Null => PilotValue.Null,
            PilotValueKind.Undefined => PilotValue.Undefined,
            PilotValueKind.Bool => PilotValue.FromBool(reader.ReadBoolean()),
            PilotValueKind.Int => PilotValue.FromInt(reader.ReadInt64()),
            PilotValueKind.UInt => PilotValue.FromUInt(reader.ReadUInt64()),
            PilotValueKind.Float => PilotValue.FromFloat(reader.ReadDouble()),
            PilotValueKind.Decimal => PilotValue.FromDecimal(reader.ReadDecimal()),
            PilotValueKind.Char => PilotValue.FromChar(reader.ReadChar()),
            PilotValueKind.String => PilotValue.FromString(reader.ReadString()),
            PilotValueKind.List => ReadList(reader),
            PilotValueKind.Tuple => ReadTuple(reader),
            PilotValueKind.Range => ReadRange(reader),
            _ => PilotValue.Null
        };
    }

    private static PilotValue ReadList(BinaryReader reader)
    {
        int count = reader.ReadInt32();
        var list = new List<PilotValue>(count);
        for (int i = 0; i < count; i++)
        {
            list.Add(ReadValue(reader));
        }
        return PilotValue.FromList(list);
    }

    private static PilotValue ReadTuple(BinaryReader reader)
    {
        int count = reader.ReadInt32();
        var arr = new PilotValue[count];
        for (int i = 0; i < count; i++)
        {
            arr[i] = ReadValue(reader);
        }
        return PilotValue.FromTuple(arr);
    }

    private static PilotValue ReadRange(BinaryReader reader)
    {
        long start = reader.ReadInt64();
        long end = reader.ReadInt64();
        long step = reader.ReadInt64();
        bool inc = reader.ReadBoolean();
        return PilotValue.FromRange(new PilotRange(start, end, step, inc));
    }

    #endregion
}
