using System;
using System.Linq;
using System.Text;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Builtins;

/// <summary>
/// Source generator that generates fluent '.with_<field>(val)' methods for classes.
/// Triggered by @generate("builder") or @builder.
/// </summary>
public sealed class BuilderGenerator : IPilotCodeGenerator
{
    public string Name => "builder";

    public void Execute(PilotGeneratorContext context)
    {
        foreach (var cls in context.Script.Classes)
        {
            if (!IsTargetClass(cls)) continue;

            var fields = cls.Fields;
            if (fields.Count == 0) continue;

            var sb = new StringBuilder();
            sb.AppendLine($"partial class {cls.Name} {{");

            foreach (var field in fields)
            {
                string typeAnnot = !string.IsNullOrEmpty(field.TypeName) ? $": {field.TypeName}" : "";
                sb.AppendLine($"    def with_{field.Name}(self, val{typeAnnot}) -> {cls.Name} {{");
                sb.AppendLine($"        self.{field.Name} = val");
                sb.AppendLine("        return self");
                sb.AppendLine("    }");
            }

            sb.AppendLine("}");

            cls.AddPartial(sb.ToString());
            context.ReportDiagnostic($"[CodeGen:Builder] Generated fluent builder methods for '{cls.Name}'.");
        }
    }

    private static bool IsTargetClass(ClassModel cls)
    {
        return cls.Decorators.Any(d =>
            string.Equals(d.Name, "builder", StringComparison.OrdinalIgnoreCase) ||
            (string.Equals(d.Name, "generate", StringComparison.OrdinalIgnoreCase) &&
             d.PositionalArgs.Any(a => a.Trim('\"', '\'').Equals("builder", StringComparison.OrdinalIgnoreCase))));
    }
}
