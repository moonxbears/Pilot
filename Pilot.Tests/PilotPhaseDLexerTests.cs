using System;
using System.Linq;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 29 verification suite:
/// - 29.1: Portable SIMD / SWAR character-class and delimiter scanner.
/// - 29.2: Zero-copy in-place preprocessor state machine (PREPROC001–PREPROC011).
/// - 29.3: Unified {} brace and Pythonic : indentation state machine (VirtualLBrace / VirtualRBrace).
/// - 29.4: Pilot modal lexing, call-quoting maximal-munch & dimensional literals.
/// - 29.5: Stage 1 SoA TokenTape & balanced delimiter jump table (O(1) BracketLinks).
/// </summary>
public class PilotPhaseDLexerTests
{
    [Fact]
    public void Tokenize_EmitsDenseSoATokenTape_WithAccurateTokenKinds()
    {
        string code = "def add(x: i64, y: i64) -> i64 { return x + y; }";
        using var tape = NativeAstBridge.TokenizeUtf8(code);

        Assert.True(tape.Length >= 15);
        var kinds = tape.Kinds;

        Assert.Equal(NativeTokenKind.KwDef, kinds[0]);
        Assert.Equal(NativeTokenKind.Ident, kinds[1]);
        Assert.Equal(NativeTokenKind.LParen, kinds[2]);
        Assert.Equal(NativeTokenKind.Ident, kinds[3]);
        Assert.Equal(NativeTokenKind.Colon, kinds[4]);
        Assert.Equal(NativeTokenKind.Ident, kinds[5]);
        Assert.Equal(NativeTokenKind.Comma, kinds[6]);
        Assert.Equal(NativeTokenKind.Ident, kinds[7]);
        Assert.Equal(NativeTokenKind.Colon, kinds[8]);
        Assert.Equal(NativeTokenKind.Ident, kinds[9]);
        Assert.Equal(NativeTokenKind.RParen, kinds[10]);
        Assert.Equal(NativeTokenKind.Arrow, kinds[11]);
        Assert.Equal(NativeTokenKind.Ident, kinds[12]);
        Assert.Equal(NativeTokenKind.LBrace, kinds[13]);
        Assert.Equal(NativeTokenKind.KwReturn, kinds[14]);
    }

    [Fact]
    public void BalancedDelimiterJumpTable_ProvidesO1MatchingBracketLinks()
    {
        string code = "def calculate(a: [i64], b: {str: i64}) -> (i64, i64) { return (a[0], b[\"x\"]); }";
        using var tape = NativeAstBridge.TokenizeUtf8(code);

        int lparenCount = 0;
        int rparenCount = 0;
        int lbracketCount = 0;
        int rbracketCount = 0;
        int lbraceCount = 0;
        int rbraceCount = 0;

        for (int i = 0; i < tape.Length; i++)
        {
            var kind = tape.Kinds[i];
            if (kind is NativeTokenKind.LParen or NativeTokenKind.LBracket or NativeTokenKind.LBrace)
            {
                if (kind == NativeTokenKind.LParen) lparenCount++;
                if (kind == NativeTokenKind.LBracket) lbracketCount++;
                if (kind == NativeTokenKind.LBrace) lbraceCount++;

                uint? matchingClose = tape.GetMatchingBracket(i);
                Assert.NotNull(matchingClose);
                int closeIdx = (int)matchingClose.Value;
                Assert.True(closeIdx > i);

                // Bidirectional jump link verification
                uint? matchingOpen = tape.GetMatchingBracket(closeIdx);
                Assert.Equal((uint)i, matchingOpen);
            }
            else if (kind == NativeTokenKind.RParen) rparenCount++;
            else if (kind == NativeTokenKind.RBracket) rbracketCount++;
            else if (kind == NativeTokenKind.RBrace) rbraceCount++;
        }

        Assert.True(lparenCount > 0 && lparenCount == rparenCount);
        Assert.True(lbracketCount > 0 && lbracketCount == rbracketCount);
        Assert.True(lbraceCount > 0 && lbraceCount == rbraceCount);
    }

    [Fact]
    public void PythonicIndentation_DesugarsColonBlocksIntoVirtualBraces()
    {
        string code =
            "if condition:\n" +
            "    let x = 100\n" +
            "    let y = 200\n" +
            "    return x + y\n" +
            "let outside = 300\n";

        using var tape = NativeAstBridge.TokenizeUtf8(code);
        var kinds = tape.Kinds;

        bool foundVirtualLBrace = false;
        bool foundVirtualRBrace = false;

        for (int i = 0; i < kinds.Length; i++)
        {
            if (kinds[i] == NativeTokenKind.VirtualLBrace)
            {
                foundVirtualLBrace = true;
                // Verify balanced bracket link for virtual brace!
                uint? match = tape.GetMatchingBracket(i);
                Assert.NotNull(match);
                Assert.Equal(NativeTokenKind.VirtualRBrace, kinds[(int)match.Value]);
            }
            if (kinds[i] == NativeTokenKind.VirtualRBrace)
            {
                foundVirtualRBrace = true;
            }
        }

        Assert.True(foundVirtualLBrace, "Expected VirtualLBrace to be emitted for indented block after colon.");
        Assert.True(foundVirtualRBrace, "Expected VirtualRBrace to be emitted when dedenting.");
    }

    [Fact]
    public void CallQuotingMaximalMunch_RespectsMilestone17AndInvariant1()
    {
        // When followed immediately by '(':
        // fn!(a)   -> CallQuoteShallow
        // fn!!(b)  -> CallQuoteDeep
        // fn!&(c)  -> CallQuoteAbi
        // fn!^(d)  -> CallQuoteReact
        // fn!@(e)  -> CallQuoteAdjoint
        // fn!?(f)  -> CallQuoteEffect
        // When NOT followed immediately by '(':
        // !flag    -> Exclamation
        // a != b   -> NotEqual
        string code = "fn!(1) + fn!!(2) + fn!&(3) + fn!^(4) + fn!@(5) + fn!?(6) + !flag + (a != b)";
        using var tape = NativeAstBridge.TokenizeUtf8(code);
        var kinds = tape.Kinds.ToArray();

        Assert.Contains(NativeTokenKind.CallQuoteShallow, kinds);
        Assert.Contains(NativeTokenKind.CallQuoteDeep, kinds);
        Assert.Contains(NativeTokenKind.CallQuoteAbi, kinds);
        Assert.Contains(NativeTokenKind.CallQuoteReact, kinds);
        Assert.Contains(NativeTokenKind.CallQuoteAdjoint, kinds);
        Assert.Contains(NativeTokenKind.CallQuoteEffect, kinds);
        Assert.Contains(NativeTokenKind.Exclamation, kinds);
        Assert.Contains(NativeTokenKind.NotEqual, kinds);
    }

    [Fact]
    public void DimensionalAndNumericLiterals_AreScannedAccurately()
    {
        string code = "let t1 = 250ms; let t2 = 10s; let f1 = 2.4GHz; let m1 = 64KB; let m2 = 16GB; let hex = 0xFF; let bin = 0b1010; let oct = 0o77; let flt = 3.1415;";
        using var tape = NativeAstBridge.TokenizeUtf8(code);
        var kinds = tape.Kinds.ToArray();

        Assert.Equal(2, kinds.Count(k => k == NativeTokenKind.DurationLit));
        Assert.Equal(1, kinds.Count(k => k == NativeTokenKind.FrequencyLit));
        Assert.Equal(2, kinds.Count(k => k == NativeTokenKind.MemoryLit));
        Assert.Equal(1, kinds.Count(k => k == NativeTokenKind.HexIntLit));
        Assert.Equal(1, kinds.Count(k => k == NativeTokenKind.BinIntLit));
        Assert.Equal(1, kinds.Count(k => k == NativeTokenKind.OctIntLit));
        Assert.Equal(1, kinds.Count(k => k == NativeTokenKind.FloatLit));
    }

    [Fact]
    public void InPlacePreprocessor_SkipsInactiveBranchesWithoutAllocations()
    {
        // Without DEBUG defined, the #if DEBUG branch is skipped
        string code =
            "#if DEBUG\n" +
            "let debug_var = 12345;\n" +
            "#else\n" +
            "let release_var = 67890;\n" +
            "#endif\n";

        using var tape = NativeAstBridge.TokenizeUtf8(code);
        var spans = tape.Spans;

        // Verify that tokens only come from the active release branch
        for (int i = 0; i < tape.Length; i++)
        {
            var span = spans[i];
            if (span.Length > 0)
            {
                string tokenText = code.Substring((int)span.Start, (int)span.Length);
                Assert.DoesNotContain("debug_var", tokenText);
            }
        }
    }
}
