using System;
using System.Linq;
using System.Text;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Builtins;

/// <summary>
/// Source generator that generates clean '.to_string()' representation for classes.
/// Triggered by @generate("repr"), @repr, or @dataclass.
/// </summary>
public sealed class ReprGenerator : IPilotCodeGenerator
{
    public string Name => "repr";

    public void Execute(PilotGeneratorContext context)
    {
        foreach (var cls in context.Script.Classes)
        {
            if (!IsTargetClass(cls)) continue;

            var fields = cls.Fields;
            if (fields.Count == 0) continue;

            var sb = new StringBuilder();
            sb.AppendLine($"partial class {cls.Name} {{");

            sb.AppendLine("    def to_string(self) -> str {");
            sb.Append($"        return \"{cls.Name}(\" + ");
            var parts = fields.Select(f => $"\"{f.Name}=\" + str(self.{f.Name})");
            sb.Append(string.Join(" + \", \" + ", parts));
            sb.AppendLine(" + \")\"");
            sb.AppendLine("    }");

            sb.AppendLine("}");

            cls.AddPartial(sb.ToString());
            context.ReportDiagnostic($"[CodeGen:Repr] Generated to_string() for '{cls.Name}'.");
        }
    }

    private static bool IsTargetClass(ClassModel cls)
    {
        return cls.Decorators.Any(d =>
            string.Equals(d.Name, "repr", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(d.Name, "dataclass", StringComparison.OrdinalIgnoreCase) ||
            (string.Equals(d.Name, "generate", StringComparison.OrdinalIgnoreCase) &&
             d.PositionalArgs.Any(a =>
                 a.Trim('\"', '\'').Equals("repr", StringComparison.OrdinalIgnoreCase) ||
                 a.Trim('\"', '\'').Equals("to_string", StringComparison.OrdinalIgnoreCase))));
    }
}
