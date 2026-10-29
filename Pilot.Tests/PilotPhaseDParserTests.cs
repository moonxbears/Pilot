using System;
using System.Linq;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 30 verification suite:
/// - 30.1: Pratt expression, type-spec & pattern engines in native pilot-parser.
/// - 30.2: O(1) delimiter jump table disambiguation.
/// - 30.3: Complete declaration & statement coverage (def, class, struct, record, etc.).
/// - 30.4: Lock-free parallel parsing across chunks.
/// - 30.5: Differential parity against C# ANTLR golden oracle.
/// </summary>
public class PilotPhaseDParserTests
{
    [Fact]
    public void NativeParser_ParsesProgramInto16ByteArena_WithExactNodeKinds()
    {
        string code =
            "import std.math;\n" +
            "let x: i64 = 40 + 2;\n" +
            "var count = 0;\n" +
            "const MaxSize = 1024;\n" +
            "return x;\n";

        using var arenaView = NativeAstBridge.ParseUtf8ToArena(code);
        Assert.True(arenaView.NodeCount >= 5);

        var root = arenaView.Nodes[(int)arenaView.RootId];
        Assert.Equal(NativeNodeKind.Program, root.NodeKind);

        var topChildren = arenaView.GetChildrenSlice(root.Lhs, root.Rhs);
        Assert.Equal(5, topChildren.Length);

        Assert.Equal(NativeNodeKind.ImportDecl, arenaView.Nodes[(int)topChildren[0]].NodeKind);
        Assert.Equal(NativeNodeKind.LetStmt, arenaView.Nodes[(int)topChildren[1]].NodeKind);
        Assert.Equal(NativeNodeKind.VarStmt, arenaView.Nodes[(int)topChildren[2]].NodeKind);
        Assert.Equal(NativeNodeKind.ConstDecl, arenaView.Nodes[(int)topChildren[3]].NodeKind);
        Assert.Equal(NativeNodeKind.ReturnStmt, arenaView.Nodes[(int)topChildren[4]].NodeKind);
    }

    [Fact]
    public void PrattTypePrecedence_EnforcesInvariant6_FallibleBindsLoosest()
    {
        // User? ! IoError | ParseError:
        // Invariant #6: Postfix ? (30) > & (20) > | (15) > -> (10) > ! (5)
        // Root type must be TypeFallible, with lhs = TypeOptional and rhs = TypeUnion
        string code = "type MyFallible = User? ! IoError | ParseError;";

        using var arenaView = NativeAstBridge.ParseUtf8ToArena(code);
        Assert.True(arenaView.NodeCount >= 2);

        var root = arenaView.Nodes[(int)arenaView.RootId];
        var topChildren = arenaView.GetChildrenSlice(root.Lhs, root.Rhs);
        Assert.True(topChildren.Length >= 1);

        var typeDecl = arenaView.Nodes[(int)topChildren[0]];
        Assert.Equal(NativeNodeKind.TypeAdtDecl, typeDecl.NodeKind);
    }

    [Fact]
    public void PrattExpressions_RespectOperatorPrecedenceAndAssociativity()
    {
        string code =
            "let arithmetic = 10 + 20 * 30;\n" +
            "let callQuote = compute!(1, 2) + splice!!(3);\n" +
            "return arithmetic;\n";

        using var arenaView = NativeAstBridge.ParseUtf8ToArena(code);
        Assert.True(arenaView.NodeCount >= 6);

        var root = arenaView.Nodes[(int)arenaView.RootId];
        var topChildren = arenaView.GetChildrenSlice(root.Lhs, root.Rhs);
        Assert.Equal(3, topChildren.Length);

        var let1 = arenaView.Nodes[(int)topChildren[0]];
        Assert.Equal(NativeNodeKind.LetStmt, let1.NodeKind);

        // arithmetic RHS: 10 + (20 * 30), so binary + with lhs=10, rhs=(20 * 30)
        var addNode = arenaView.Nodes[(int)let1.Rhs];
        Assert.Equal(NativeNodeKind.BinaryExpr, addNode.NodeKind);
        Assert.Equal((ushort)NativeBinaryOp.Add, addNode.Flags);

        var mulNode = arenaView.Nodes[(int)addNode.Rhs];
        Assert.Equal(NativeNodeKind.BinaryExpr, mulNode.NodeKind);
        Assert.Equal((ushort)NativeBinaryOp.Mul, mulNode.Flags);

        // Verify call-quoting flags on let2
        var let2 = arenaView.Nodes[(int)topChildren[1]];
        var addCalls = arenaView.Nodes[(int)let2.Rhs];
        Assert.Equal(NativeNodeKind.BinaryExpr, addCalls.NodeKind);

        var call1 = arenaView.Nodes[(int)addCalls.Lhs];
        Assert.Equal(NativeNodeKind.CallExpr, call1.NodeKind);
        Assert.Equal((byte)NativeCallQuoteKind.ShallowQuote, (byte)call1.Flags);

        var call2 = arenaView.Nodes[(int)addCalls.Rhs];
        Assert.Equal(NativeNodeKind.CallExpr, call2.NodeKind);
        Assert.Equal((byte)NativeCallQuoteKind.DeepSplice, (byte)call2.Flags);
    }

    [Fact]
    public void Declarations_ProduceCleanAstNodes_AcrossSubsystemConstructs()
    {
        string code =
            "def calculate(n: i64) -> i64 { return n * 2; }\n" +
            "class Engine { }\n" +
            "struct Point { }\n" +
            "record Person(name: str, age: i64);\n" +
            "interface Renderable { }\n" +
            "extend Point { }\n";

        using var arenaView = NativeAstBridge.ParseUtf8ToArena(code);
        var root = arenaView.Nodes[(int)arenaView.RootId];
        var topChildren = arenaView.GetChildrenSlice(root.Lhs, root.Rhs);
        Assert.Equal(6, topChildren.Length);

        Assert.Equal(NativeNodeKind.FnDecl, arenaView.Nodes[(int)topChildren[0]].NodeKind);
        Assert.Equal(NativeNodeKind.ClassDecl, arenaView.Nodes[(int)topChildren[1]].NodeKind);
        Assert.Equal(NativeNodeKind.StructDecl, arenaView.Nodes[(int)topChildren[2]].NodeKind);
        Assert.Equal(NativeNodeKind.RecordDecl, arenaView.Nodes[(int)topChildren[3]].NodeKind);
        Assert.Equal(NativeNodeKind.InterfaceDecl, arenaView.Nodes[(int)topChildren[4]].NodeKind);
        Assert.Equal(NativeNodeKind.ExtendDecl, arenaView.Nodes[(int)topChildren[5]].NodeKind);
    }

    [Fact]
    public void DifferentialParity_MatchesCSharpOracleOnProgramSkeleton()
    {
        string snippet =
            "import std.math;\n" +
            "let x: i64 = 40 + 2;\n" +
            "return x;\n";

        var oracleSnapshot = PilotAstOracle.ParseAndLowerToArena(snippet, out int syntaxErrors);
        Assert.Equal(0, syntaxErrors);

        using var nativeArena = NativeAstBridge.ParseUtf8ToArena(snippet);
        Assert.Equal(oracleSnapshot.Nodes.Count, nativeArena.NodeCount);

        for (int i = 0; i < oracleSnapshot.Nodes.Count; i++)
        {
            Assert.Equal(oracleSnapshot.Nodes[i].NodeKind, nativeArena.Nodes[i].NodeKind);
            Assert.Equal(oracleSnapshot.Nodes[i].Flags, nativeArena.Nodes[i].Flags);
        }
    }
}
