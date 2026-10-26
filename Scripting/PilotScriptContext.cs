using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Scripting;

/// <summary>
/// Configures and populates a Pilot execution scope with first-class scripting capabilities:
/// - Command-line arguments via 'args' and 'argv'
/// - System environment access via 'env'
/// - Working directory query via 'cwd()'
/// - Script termination via 'exit(code)'
/// - File system utilities: 'read_text(path)', 'write_text(path, content)', 'file_exists(path)'
/// </summary>
public static class PilotScriptContext
{
    /// <summary>
    /// Injects scripting environment globals into the provided execution context.
    /// </summary>
    public static void Inject(
        PilotExecutionContext context,
        IReadOnlyList<string>? scriptArgs = null,
        string? scriptPath = null)
    {
        var scope = context.Environment.GlobalScope;

        // 1. Command-line arguments: args and argv
        var argsList = new List<PilotValue>();
        if (scriptArgs != null)
        {
            foreach (var arg in scriptArgs)
            {
                argsList.Add(PilotValue.FromString(arg));
            }
        }
        var argsVal = PilotValue.FromList(argsList);
        scope.Declare("args", argsVal);
        scope.Declare("argv", argsVal);

        // 2. Script path
        scope.Declare("script_path", PilotValue.FromString(scriptPath ?? string.Empty));

        // 3. Environment variables: env
        var envMap = new Dictionary<PilotValue, PilotValue>();
        foreach (DictionaryEntry entry in System.Environment.GetEnvironmentVariables())
        {
            string k = entry.Key?.ToString() ?? string.Empty;
            string v = entry.Value?.ToString() ?? string.Empty;
            envMap[PilotValue.FromString(k)] = PilotValue.FromString(v);
        }
        scope.Declare("env", PilotValue.FromMap(envMap));

        // 4. cwd(): Current working directory
        scope.Declare("cwd", PilotValue.FromFunction(PilotFunction.CreateNative(
            "cwd",
            (ctx, args) => PilotValue.FromString(Directory.GetCurrentDirectory()))));

        // 5. exit(code): Script termination
        scope.Declare("exit", PilotValue.FromFunction(PilotFunction.CreateNative(
            "exit",
            (ctx, args) =>
            {
                int exitCode = args.Count > 0 ? (int)args[0].AsInt() : 0;
                throw new PilotScriptExitException(exitCode);
            },
            "code")));

        // 6. read_text(path): Reads entire file as text
        scope.Declare("read_text", PilotValue.FromFunction(PilotFunction.CreateNative(
            "read_text",
            (ctx, args) =>
            {
                if (args.Count == 0) return PilotValue.FromString(string.Empty);
                string path = args[0].AsString();
                if (!File.Exists(path)) return PilotValue.FromString(string.Empty);
                return PilotValue.FromString(File.ReadAllText(path));
            },
            "path")));

        // 7. write_text(path, content): Writes text content to file
        scope.Declare("write_text", PilotValue.FromFunction(PilotFunction.CreateNative(
            "write_text",
            (ctx, args) =>
            {
                if (args.Count < 2) return PilotValue.False;
                string path = args[0].AsString();
                string content = args[1].AsString();
                File.WriteAllText(path, content);
                return PilotValue.True;
            },
            "path", "content")));

        // 8. file_exists(path): Checks if file exists
        scope.Declare("file_exists", PilotValue.FromFunction(PilotFunction.CreateNative(
            "file_exists",
            (ctx, args) =>
            {
                if (args.Count == 0) return PilotValue.False;
                string path = args[0].AsString();
                return PilotValue.FromBool(File.Exists(path));
            },
            "path")));

        // 9. delete_file(path): Deletes a file if present
        scope.Declare("delete_file", PilotValue.FromFunction(PilotFunction.CreateNative(
            "delete_file",
            (ctx, args) =>
            {
                if (args.Count == 0) return PilotValue.False;
                string path = args[0].AsString();
                if (File.Exists(path))
                {
                    File.Delete(path);
                    return PilotValue.True;
                }
                return PilotValue.False;
            },
            "path")));

        // 10. println(args...): Convenience println
        scope.Declare("println", PilotValue.FromFunction(PilotFunction.CreateNative(
            "println",
            (ctx, args) =>
            {
                string text = string.Join(" ", args);
                ctx.Options.StandardOutput.WriteLine(text);
                return PilotValue.Undefined;
            })));
    }
}
