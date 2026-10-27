using System;
using System.Linq;
using System.Text;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Builtins;

/// <summary>
/// Source generator that generates '.equals(other)' and '.hash()' for classes.
/// Triggered by @generate("equality") or @equality.
/// </summary>
public sealed class EqualityGenerator : IPilotCodeGenerator
{
    public string Name => "equality";

    public void Execute(PilotGeneratorContext context)
    {
        foreach (var cls in context.Script.Classes)
        {
            if (!IsTargetClass(cls)) continue;

            var fields = cls.Fields;
            if (fields.Count == 0) continue;

            var sb = new StringBuilder();
            sb.AppendLine($"partial class {cls.Name} {{");

            // def equals(self, other: any) -> bool
            sb.AppendLine("    def equals(self, other: any) -> bool {");
            sb.AppendLine("        guard other is not None else { return false }");
            var comparisons = fields.Select(f => $"self.{f.Name} == other.{f.Name}");
            sb.AppendLine($"        return {string.Join(" && ", comparisons)}");
            sb.AppendLine("    }");

            // def hash(self) -> int
            sb.AppendLine("    def hash(self) -> int {");
            sb.Append("        return ");
            var hashes = fields.Select(f => $"int(self.{f.Name})");
            sb.Append(string.Join(" ^ ", hashes));
            sb.AppendLine();
            sb.AppendLine("    }");

            sb.AppendLine("}");

            cls.AddPartial(sb.ToString());
            context.ReportDiagnostic($"[CodeGen:Equality] Generated equals() and hash() for '{cls.Name}'.");
        }
    }

    private static bool IsTargetClass(ClassModel cls)
    {
        return cls.Decorators.Any(d =>
            string.Equals(d.Name, "equality", StringComparison.OrdinalIgnoreCase) ||
            (string.Equals(d.Name, "generate", StringComparison.OrdinalIgnoreCase) &&
             d.PositionalArgs.Any(a => a.Trim('\"', '\'').Equals("equality", StringComparison.OrdinalIgnoreCase))));
    }
}
