using System;
using System.Linq;
using System.Text;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Builtins;

/// <summary>
/// Source generator that generates property notification mechanism.
/// Like C# INotifyPropertyChanged / Vue reactivity, but straightforward and Pythonic:
/// - cls.subscribe(callback)
/// - cls.notify_change(prop_name, old_val, new_val)
/// - Setter mutators that automatically notify listeners.
/// Triggered by @generate("observable") or @observable.
/// </summary>
public sealed class ObservableGenerator : IPilotCodeGenerator
{
    public string Name => "observable";

    public void Execute(PilotGeneratorContext context)
    {
        foreach (var cls in context.Script.Classes)
        {
            if (!IsTargetClass(cls)) continue;

            var sb = new StringBuilder();
            sb.AppendLine($"partial class {cls.Name} {{");
            sb.AppendLine("    var _change_listeners: List = []");
            sb.AppendLine();
            sb.AppendLine("    def subscribe(self, callback) {");
            sb.AppendLine("        self._change_listeners.append(callback)");
            sb.AppendLine("    }");
            sb.AppendLine();
            sb.AppendLine("    def notify_change(self, prop_name: str, old_val: any, new_val: any) {");
            sb.AppendLine("        var i: int = 0");
            sb.AppendLine("        let n: int = len(self._change_listeners)");
            sb.AppendLine("        while (i < n) {");
            sb.AppendLine("            let cb = self._change_listeners[i]");
            sb.AppendLine("            cb(self, prop_name, old_val, new_val)");
            sb.AppendLine("            i += 1");
            sb.AppendLine("        }");
            sb.AppendLine("    }");

            // Generate observable mutators: set_<prop>(val)
            foreach (var field in cls.Fields.Where(f => f.IsMutable))
            {
                sb.AppendLine();
                sb.AppendLine($"    def set_{field.Name}(self, new_val) {{");
                sb.AppendLine($"        let old_val = self.{field.Name}");
                sb.AppendLine($"        self.{field.Name} = new_val");
                sb.AppendLine($"        self.notify_change(\"{field.Name}\", old_val, new_val)");
                sb.AppendLine("    }");
            }

            sb.AppendLine("}");

            cls.AddPartial(sb.ToString());
            context.ReportDiagnostic($"[CodeGen:Observable] Generated reactive subscription and setter events for '{cls.Name}'.");
        }
    }

    private static bool IsTargetClass(ClassModel cls)
    {
        return cls.Decorators.Any(d =>
            string.Equals(d.Name, "observable", StringComparison.OrdinalIgnoreCase) ||
            (string.Equals(d.Name, "generate", StringComparison.OrdinalIgnoreCase) &&
             d.PositionalArgs.Any(a => a.Trim('\"', '\'').Equals("observable", StringComparison.OrdinalIgnoreCase))));
    }
}
