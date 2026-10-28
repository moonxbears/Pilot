using System;
using System.Collections.Generic;
using System.IO;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Bytecode;

/// <summary>
/// Dense O(1) jump table used by <see cref="PilotOpcode.JumpTable"/> for ADT tag and integer interval dispatch.
/// </summary>
public sealed class PilotJumpTable
{
    public long MinValue { get; set; }
    public int[] Targets { get; set; }
    public int DefaultTarget { get; set; }

    public PilotJumpTable(long minValue, int[] targets, int defaultTarget)
    {
        MinValue = minValue;
        Targets = targets;
        DefaultTarget = defaultTarget;
    }
}

/// <summary>
/// Serializable field metadata for an ADT variant or Record product type in `.pilc` bytecode chunks.
/// </summary>
public sealed class PilotSerializedFieldDescriptor
{
    public string? Name { get; set; }
    public string TypeName { get; set; } = "any";
}

/// <summary>
/// Serializable variant metadata for an ADT sum type in `.pilc` bytecode chunks.
/// </summary>
public sealed class PilotSerializedVariantDescriptor
{
    public string Name { get; set; } = "";
    public List<PilotSerializedFieldDescriptor> Fields { get; set; } = new();
}

/// <summary>
/// Serializable ADT or Record descriptor persisted inside `.pilc` bytecode chunks so standalone
/// binary execution reconstructs exact memory layouts and constructors without AST access.
/// </summary>
public sealed class PilotBytecodeTypeDescriptor
{
    public bool IsRecord { get; set; }
    public string Name { get; set; } = "";
    public List<string> GenericParameters { get; set; } = new();
    public List<PilotSerializedVariantDescriptor> Variants { get; set; } = new();
    public List<PilotSerializedFieldDescriptor> RecordFields { get; set; } = new();
}

/// <summary>
/// Encapsulates a compiled sequence of bytecode instructions, constant pool,
/// variable bindings, local slot layout, O(1) jump tables, ADT/record descriptors,
/// and nested closure chunks.
/// </summary>
public sealed class PilotBytecodeChunk
{
    private const int ChunkFormatV2Marker = -0x50494C32; // Distinguishes M11 extended tail cleanly if needed

    public string Name { get; set; } = "<main>";
    public List<PilotInstruction> Instructions { get; } = new();
    public List<PilotValue> Constants { get; } = new();
    public List<string> IdentifierPool { get; } = new();
    public List<string> ParameterNames { get; } = new();
    public List<PilotBytecodeChunk> NestedChunks { get; } = new();
    public Dictionary<string, int> LocalSlotMap { get; } = new(StringComparer.Ordinal);
    public int LocalCount { get; set; } = 0;
    public List<IParseTree> AstFallbackNodes { get; } = new();
    public List<PilotJumpTable> JumpTables { get; } = new();
    public List<PilotBytecodeTypeDescriptor> TypeDescriptors { get; } = new();
    public PilotParser.Func_defContext? FuncDefContext { get; set; }

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
    /// Adds a dense O(1) jump table to the chunk, returning its index.
    /// </summary>
    public int AddJumpTable(PilotJumpTable table)
    {
        JumpTables.Add(table);
        return JumpTables.Count - 1;
    }

    /// <summary>
    /// Adds an ADT or Record descriptor to the chunk, returning its index.
    /// </summary>
    public int AddTypeDescriptor(PilotBytecodeTypeDescriptor descriptor)
    {
        TypeDescriptors.Add(descriptor);
        return TypeDescriptors.Count - 1;
    }

    /// <summary>
    /// Adds a value to the constant pool, returning its index. Reuses identical constants when possible.
    /// </summary>
    public int AddConstant(PilotValue value)
    {
        for (int i = 0; i < Constants.Count; i++)
        {
            if (Constants[i].Kind == value.Kind && Constants[i].Equals(value))
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

        // Milestone 11 Extension Section: JumpTables & TypeDescriptors
        writer.Write(ChunkFormatV2Marker);
        writer.Write(JumpTables.Count);
        foreach (var jt in JumpTables)
        {
            writer.Write(jt.MinValue);
            writer.Write(jt.DefaultTarget);
            writer.Write(jt.Targets.Length);
            for (int i = 0; i < jt.Targets.Length; i++)
            {
                writer.Write(jt.Targets[i]);
            }
        }

        writer.Write(TypeDescriptors.Count);
        foreach (var td in TypeDescriptors)
        {
            writer.Write(td.IsRecord);
            writer.Write(td.Name);
            writer.Write(td.GenericParameters.Count);
            foreach (var gp in td.GenericParameters) writer.Write(gp);

            writer.Write(td.Variants.Count);
            foreach (var v in td.Variants)
            {
                writer.Write(v.Name);
                writer.Write(v.Fields.Count);
                foreach (var f in v.Fields)
                {
                    writer.Write(f.Name ?? string.Empty);
                    writer.Write(f.TypeName);
                }
            }

            writer.Write(td.RecordFields.Count);
            foreach (var rf in td.RecordFields)
            {
                writer.Write(rf.Name ?? string.Empty);
                writer.Write(rf.TypeName);
            }
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

        // Milestone 11 Extension Section (gracefully backward-compatible if reading V1 stream at EOF)
        if (reader.BaseStream.CanSeek && reader.BaseStream.Position + 4 <= reader.BaseStream.Length)
        {
            long pos = reader.BaseStream.Position;
            int marker = reader.ReadInt32();
            if (marker == ChunkFormatV2Marker)
            {
                ReadV2Extensions(reader, chunk);
            }
            else
            {
                reader.BaseStream.Position = pos;
            }
        }
        else if (!reader.BaseStream.CanSeek)
        {
            try
            {
                int marker = reader.ReadInt32();
                if (marker == ChunkFormatV2Marker)
                {
                    ReadV2Extensions(reader, chunk);
                }
            }
            catch (EndOfStreamException)
            {
            }
        }

        return chunk;
    }

    private static void ReadV2Extensions(BinaryReader reader, PilotBytecodeChunk chunk)
    {
        int jtCount = reader.ReadInt32();
        for (int i = 0; i < jtCount; i++)
        {
            long minVal = reader.ReadInt64();
            int defaultTarget = reader.ReadInt32();
            int len = reader.ReadInt32();
            var targets = new int[len];
            for (int t = 0; t < len; t++)
            {
                targets[t] = reader.ReadInt32();
            }
            chunk.JumpTables.Add(new PilotJumpTable(minVal, targets, defaultTarget));
        }

        int tdCount = reader.ReadInt32();
        for (int i = 0; i < tdCount; i++)
        {
            var td = new PilotBytecodeTypeDescriptor
            {
                IsRecord = reader.ReadBoolean(),
                Name = reader.ReadString()
            };
            int gpCount = reader.ReadInt32();
            for (int g = 0; g < gpCount; g++) td.GenericParameters.Add(reader.ReadString());

            int varCount = reader.ReadInt32();
            for (int v = 0; v < varCount; v++)
            {
                var vd = new PilotSerializedVariantDescriptor { Name = reader.ReadString() };
                int fCount = reader.ReadInt32();
                for (int f = 0; f < fCount; f++)
                {
                    string fn = reader.ReadString();
                    string ft = reader.ReadString();
                    vd.Fields.Add(new PilotSerializedFieldDescriptor
                    {
                        Name = string.IsNullOrEmpty(fn) ? null : fn,
                        TypeName = ft
                    });
                }
                td.Variants.Add(vd);
            }

            int rfCount = reader.ReadInt32();
            for (int r = 0; r < rfCount; r++)
            {
                string fn = reader.ReadString();
                string ft = reader.ReadString();
                td.RecordFields.Add(new PilotSerializedFieldDescriptor
                {
                    Name = string.IsNullOrEmpty(fn) ? null : fn,
                    TypeName = ft
                });
            }

            chunk.TypeDescriptors.Add(td);
        }
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
            case PilotValueKind.Keyword:
                writer.Write(value.AsKeyword().FullName);
                break;
            case PilotValueKind.KeywordVariant:
            {
                var kv = value.AsKeywordVariant();
                writer.Write(kv.Keyword.FullName);
                writer.Write(kv.Fields.Length);
                foreach (var a in kv.Fields) WriteValue(writer, a);
                break;
            }
            case PilotValueKind.List:
            {
                var list = value.AsList();
                int encodedCount = value.IsFrozen ? -(list.Count + 1) : list.Count;
                writer.Write(encodedCount);
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
            case PilotValueKind.Map:
            {
                var map = value.AsMap();
                writer.Write(value.IsFrozen);
                writer.Write(map.Count);
                foreach (var kvp in map)
                {
                    WriteValue(writer, kvp.Key);
                    WriteValue(writer, kvp.Value);
                }
                break;
            }
            case PilotValueKind.Set:
            {
                var set = value.AsSet();
                writer.Write(value.IsFrozen);
                writer.Write(set.Count);
                foreach (var item in set) WriteValue(writer, item);
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
            PilotValueKind.Keyword => PilotValue.FromKeyword(reader.ReadString()),
            PilotValueKind.KeywordVariant => ReadKeywordVariant(reader),
            PilotValueKind.List => ReadList(reader),
            PilotValueKind.Tuple => ReadTuple(reader),
            PilotValueKind.Map => ReadMap(reader),
            PilotValueKind.Set => ReadSet(reader),
            PilotValueKind.Range => ReadRange(reader),
            _ => PilotValue.Null
        };
    }

    private static PilotValue ReadKeywordVariant(BinaryReader reader)
    {
        string kw = reader.ReadString();
        int count = reader.ReadInt32();
        var args = new PilotValue[count];
        for (int i = 0; i < count; i++) args[i] = ReadValue(reader);
        return PilotValue.FromKeywordVariant(new PilotKeywordVariant(PilotKeyword.Intern(kw), args));
    }

    private static PilotValue ReadList(BinaryReader reader)
    {
        int rawCount = reader.ReadInt32();
        bool isFrozen = rawCount < 0;
        int count = isFrozen ? (-rawCount - 1) : rawCount;
        var list = new List<PilotValue>(count);
        for (int i = 0; i < count; i++)
        {
            list.Add(ReadValue(reader));
        }
        return isFrozen ? PilotValue.FromFrozenList(list) : PilotValue.FromList(list);
    }

    private static PilotValue ReadMap(BinaryReader reader)
    {
        bool isFrozen = reader.ReadBoolean();
        int count = reader.ReadInt32();
        var dict = new Dictionary<PilotValue, PilotValue>(count);
        for (int i = 0; i < count; i++)
        {
            var k = ReadValue(reader);
            var v = ReadValue(reader);
            dict[k] = v;
        }
        return isFrozen ? PilotValue.FromFrozenMap(new PilotFrozenMap(dict)) : PilotValue.FromMap(dict);
    }

    private static PilotValue ReadSet(BinaryReader reader)
    {
        bool isFrozen = reader.ReadBoolean();
        int count = reader.ReadInt32();
        var set = new HashSet<PilotValue>(count);
        for (int i = 0; i < count; i++)
        {
            set.Add(ReadValue(reader));
        }
        var val = PilotValue.FromSet(set);
        return isFrozen ? val.Freeze() : val;
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
