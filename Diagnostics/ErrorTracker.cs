using System.Collections.Generic;
using Antlr4.Runtime;

namespace Pilot.Diagnostics;

/// <summary>
/// Collects syntax errors encountered by ANTLR lexer and parser.
/// </summary>
public class ErrorTracker : BaseErrorListener
{
    public List<string> Errors { get; } = new List<string>();

    public bool HasErrors => Errors.Count > 0;

    public override void SyntaxError(
        IRecognizer recognizer,
        IToken offendingSymbol,
        int line,
        int charPositionInLine,
        string msg,
        RecognitionException e)
    {
        string symbolText = offendingSymbol?.Text ?? string.Empty;
        Errors.Add($"Line {line}:{charPositionInLine} - {msg} (symbol: '{symbolText}')");
    }

    public void Clear()
    {
        Errors.Clear();
    }
}
