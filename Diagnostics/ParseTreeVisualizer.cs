using System;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;

namespace Pilot.Diagnostics;

/// <summary>
/// Renders an ANTLR parse tree with ANSI color styling and indentation.
/// </summary>
public static class ParseTreeVisualizer
{
    public static void Print(IParseTree node, Parser parser, int depth = 0)
    {
        string indent = new string(' ', depth * 2);
        string nodeText = Trees.GetNodeText(node, parser);

        if (node is ITerminalNode)
        {
            if (nodeText != "\n" && nodeText != "\r\n" && nodeText.Length > 0)
            {
                Console.WriteLine($"{indent}\u001b[90mterminal:\u001b[0m '{nodeText}'");
            }
        }
        else
        {
            Console.WriteLine($"{indent}\u001b[36m{nodeText}\u001b[0m");
            for (int i = 0; i < node.ChildCount; i++)
            {
                Print(node.GetChild(i), parser, depth + 1);
            }
        }
    }
}
