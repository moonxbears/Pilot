using System;
using System.Linq;
using System.Text;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Builtins;

/// <summary>
/// Source generator that generates '.to_json()' and '.from_json()' methods for classes.
/// Triggered by @generate("json") or @generate(json).
/// </summary>
public sealed class JsonGenerator : IPilotCodeGenerator
{
    public string Name => "json";

    public void Execute(PilotGeneratorContext context)
    {
        foreach (var cls in context.Script.Classes)
        {
            if (!IsTargetClass(cls)) continue;

            var fields = cls.Fields;
            if (fields.Count == 0) continue;

            var sb = new StringBuilder();
            sb.AppendLine($"partial class {cls.Name} {{");

            // def to_json(self) -> str
            sb.AppendLine("    def to_json(self) -> str {");
            sb.Append("        return \"{\" + ");
            var parts = fields.Select(f =>
            {
                if (f.TypeName is "str" or "string")
                {
                    return $"\"\\\"{f.Name}\\\": \\\"\" + str(self.{f.Name}) + \"\\\"\"";
                }
                return $"\"\\\"{f.Name}\\\": \" + str(self.{f.Name})";
            });
            sb.Append(string.Join(" + \", \" + ", parts));
            sb.AppendLine(" + \"}\"");
            sb.AppendLine("    }");

            // def to_map(self) -> Map
            sb.AppendLine("    def to_map(self) -> map {");
            sb.AppendLine("        return {");
            for (int i = 0; i < fields.Count; i++)
            {
                string comma = (i < fields.Count - 1) ? "," : "";
                sb.AppendLine($"            \"{fields[i].Name}\": self.{fields[i].Name}{comma}");
            }
            sb.AppendLine("        }");
            sb.AppendLine("    }");

            sb.AppendLine("}");

            cls.AddPartial(sb.ToString());
            context.ReportDiagnostic($"[CodeGen:Json] Generated JSON serializers for '{cls.Name}'.");
        }
    }

    private static bool IsTargetClass(ClassModel cls)
    {
        return cls.Decorators.Any(d =>
            string.Equals(d.Name, "json", StringComparison.OrdinalIgnoreCase) ||
            (string.Equals(d.Name, "generate", StringComparison.OrdinalIgnoreCase) &&
             d.PositionalArgs.Any(a => a.Trim('\"', '\'').Equals("json", StringComparison.OrdinalIgnoreCase))));
    }
}
