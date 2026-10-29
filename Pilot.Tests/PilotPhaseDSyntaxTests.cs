using System;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 31 verification suite:
/// - 31.1: Immutable width-relative Green tree + lazy Red cursor façade with 100% lossless round-trip invariant.
/// - 31.2: Resilient panic-free error recovery, missing-token synthesis (MissingIdent on user., MissingRParen on unclosed if), and typo fix-its.
/// - 31.3: Native Wadler-Lindig CST pretty-printer (pilot fmt port) with comment preservation, idempotency, and suite transformations.
/// </summary>
public class PilotPhaseDSyntaxTests
{
    [Fact]
    public void LosslessCstRoundTrip_Preserves100PercentOfTrivia_ByteForByte()
    {
        string code =
            "// Leading module comment\n" +
            "#region MathRoutines\n" +
            "import std.math;\n\n" +
            "/* Block comment */\n" +
            "def calculate(a: i64, b: i64) -> i64 {\n" +
            "    // Inner computation\n" +
            "    let res = (a + b) * 10;\n\n" +
            "    return res;\n" +
            "}\n" +
            "#endregion\n";

        using var cst = NativeAstBridge.ParseUtf8ToCst(code);
        Assert.Equal((nuint)0, cst.DiagnosticCount);

        string reconstructed = cst.ToText();
        Assert.Equal(code, reconstructed);
    }

    [Fact]
    public void ColonSuite_RoundTripsLosslessly()
    {
        string code =
            "def greet(name: str):\n" +
            "    let msg = \"Hello \" + name;\n" +
            "    return msg;\n";

        using var cst = NativeAstBridge.ParseUtf8ToCst(code);
        Assert.Equal((nuint)0, cst.DiagnosticCount);
        Assert.Equal(code, cst.ToText());
    }

    [Fact]
    public void ResilientErrorRecovery_SynthesizesMissingTokens_ForIdeCompletion()
    {
        // 1. Trailing dot triggers MissingIdent for IDE dot-completion
        string codeWithTrailingDot = "let x = user.\nreturn x;\n";
        using var cstDot = NativeAstBridge.ParseUtf8ToCst(codeWithTrailingDot);
        Assert.True(cstDot.HasMissingIdent);
        Assert.True(cstDot.DiagnosticCount > 0);
        // Lossless text must still preserve the original source exactly
        Assert.Equal(codeWithTrailingDot, cstDot.ToText());

        // 2. Unclosed paren before block triggers MissingRParen
        string codeWithUnclosedIf = "if (x > 0 {\n    return 1;\n}\n";
        using var cstIf = NativeAstBridge.ParseUtf8ToCst(codeWithUnclosedIf);
        Assert.True(cstIf.HasMissingRParen);
        Assert.True(cstIf.DiagnosticCount > 0);
        Assert.Equal(codeWithUnclosedIf, cstIf.ToText());
    }

    [Fact]
    public void NativeFormatter_PreservesCommentsAndIsIdempotent()
    {
        string unformatted =
            "// Leading module comment\n" +
            "def calc(a: i64, b: i64, c: i64) -> i64 {\n" +
            "    // Keep this inner comment\n" +
            "    let res = (a + b) * c;\n\n" +
            "    return res;\n" +
            "}\n";

        var (fmt1, modified1) = NativeAstBridge.FormatSourceNative(unformatted);
        Assert.NotEmpty(fmt1);
        Assert.Contains("// Leading module comment", fmt1);
        Assert.Contains("// Keep this inner comment", fmt1);
        Assert.Contains("(a + b) * c", fmt1);

        // Idempotence: formatting a second time produces identical output
        var (fmt2, modified2) = NativeAstBridge.FormatSourceNative(fmt1);
        Assert.Equal(fmt1, fmt2);
        Assert.False(modified2);
    }

    [Fact]
    public void NativeFormatter_TransformsBetweenBraceAndColonSuites()
    {
        string braceCode =
            "def check_val(x: i64) -> i64 {\n" +
            "    if x > 0 {\n" +
            "        return x;\n" +
            "    } else {\n" +
            "        return 0;\n" +
            "    }\n" +
            "}\n";

        // Transform to colon suites
        var (colonCode, _) = NativeAstBridge.FormatSourceNative(braceCode, useColonSuites: true, emitSemicolons: false);
        Assert.Contains("def check_val(x: i64) -> i64:", colonCode);
        Assert.Contains("if x > 0:", colonCode);
        Assert.Contains("else:", colonCode);

        // Round-trip back to brace suites
        var (roundTripBrace, _) = NativeAstBridge.FormatSourceNative(colonCode, useColonSuites: false, emitSemicolons: true);
        Assert.Contains("def check_val(x: i64) -> i64 {", roundTripBrace);
        Assert.Contains("if x > 0 {", roundTripBrace);
    }
}
