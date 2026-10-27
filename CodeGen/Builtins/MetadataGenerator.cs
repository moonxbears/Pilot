using System;
using System.Linq;
using System.Text;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Builtins;

/// <summary>
/// Source generator that captures @meta(k=v, ...) annotations and exposes
/// them as a '.meta' property on the target class or script.
/// </summary>
public sealed class MetadataGenerator : IPilotCodeGenerator
{
    public string Name => "metadata";

    public void Execute(PilotGeneratorContext context)
    {
        foreach (var cls in context.Script.Classes)
        {
            var metaDecs = cls.Decorators.Where(d =>
                string.Equals(d.Name, "meta", StringComparison.OrdinalIgnoreCase) ||
                string.Equals(d.Name, "metadata", StringComparison.OrdinalIgnoreCase)).ToList();

            if (metaDecs.Count == 0 && cls.Meta.Count == 0) continue;

            // Populate cls.Meta
            foreach (var dec in metaDecs)
            {
                foreach (var (k, v) in dec.NamedArgs)
                {
                    cls.Meta[k] = v.Trim('\"', '\'');
                }
                for (int i = 0; i < dec.PositionalArgs.Count; i++)
                {
                    cls.Meta[$"tag_{i}"] = dec.PositionalArgs[i].Trim('\"', '\'');
                }
            }

            var sb = new StringBuilder();
            sb.AppendLine($"partial class {cls.Name} {{");
            sb.AppendLine("    def get_meta(self) -> map {");
            sb.AppendLine("        return {");
            var entries = cls.Meta.Entries.ToList();
            for (int i = 0; i < entries.Count; i++)
            {
                string comma = (i < entries.Count - 1) ? "," : "";
                sb.AppendLine($"            \"{entries[i].Key}\": \"{entries[i].Value}\"{comma}");
            }
            sb.AppendLine("        }");
            sb.AppendLine("    }");
            sb.AppendLine("}");

            cls.AddPartial(sb.ToString());
            context.ReportDiagnostic($"[CodeGen:Metadata] Attached reflection metadata to '{cls.Name}'.");
        }
    }
}
