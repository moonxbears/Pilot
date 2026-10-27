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
    private static readonly Lazy<Dictionary<PilotValue, PilotValue>> CachedEnvTemplate = new(() =>
    {
        var envMap = new Dictionary<PilotValue, PilotValue>();
        foreach (DictionaryEntry entry in System.Environment.GetEnvironmentVariables())
        {
            string k = entry.Key?.ToString() ?? string.Empty;
            string v = entry.Value?.ToString() ?? string.Empty;
            envMap[PilotValue.FromString(k)] = PilotValue.FromString(v);
        }
        return envMap;
    });

    private static readonly PilotValue CwdFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
        "cwd",
        (ctx, args) => PilotValue.FromString(Directory.GetCurrentDirectory())));

    private static readonly PilotValue ExitFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
        "exit",
        (ctx, args) =>
        {
            int exitCode = args.Count > 0 ? (int)args[0].AsInt() : 0;
            throw new PilotScriptExitException(exitCode);
        },
        "code"));

    private static readonly PilotValue ReadTextFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
        "read_text",
        (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString(string.Empty);
            string path = args[0].AsString();
            if (!File.Exists(path)) return PilotValue.FromString(string.Empty);
            return PilotValue.FromString(File.ReadAllText(path));
        },
        "path"));

    private static readonly PilotValue WriteTextFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
        "write_text",
        (ctx, args) =>
        {
            if (args.Count < 2) return PilotValue.False;
            string path = args[0].AsString();
            string content = args[1].AsString();
            File.WriteAllText(path, content);
            return PilotValue.True;
        },
        "path", "content"));

    private static readonly PilotValue FileExistsFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
        "file_exists",
        (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            string path = args[0].AsString();
            return PilotValue.FromBool(File.Exists(path));
        },
        "path"));

    private static readonly PilotValue DeleteFileFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
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
        "path"));

    private static readonly PilotValue PrintlnFunc = PilotValue.FromFunction(PilotFunction.CreateNative(
        "println",
        (ctx, args) =>
        {
            string text = args.Count == 1 ? args[0].ToString() : string.Join(" ", args);
            ctx.Options.StandardOutput.WriteLine(text);
            return PilotValue.Undefined;
        }));

    /// <summary>
    /// Injects scripting environment globals into the provided execution context.
    /// </summary>
    public static void Inject(
        PilotExecutionContext context,
        IReadOnlyList<string>? scriptArgs = null,
        string? scriptPath = null)
    {
        var scope = context.Environment.GlobalScope;

        InjectCommandLineArgs(scope, scriptArgs);
        InjectScriptPath(scope, scriptPath);
        InjectEnvironmentVariables(scope);
        InjectProcessControlFunctions(scope);
        InjectFileSystemFunctions(scope);
        InjectUtilityFunctions(scope);
    }

    private static void InjectCommandLineArgs(PilotScope scope, IReadOnlyList<string>? scriptArgs)
    {
        // 1. Command-line arguments: args and argv
        var argsList = scriptArgs != null ? new List<PilotValue>(scriptArgs.Count) : new List<PilotValue>();
        if (scriptArgs != null)
        {
            for (int i = 0; i < scriptArgs.Count; i++)
            {
                argsList.Add(PilotValue.FromString(scriptArgs[i]));
            }
        }
        var argsVal = PilotValue.FromList(argsList);
        scope.Declare("args", argsVal);
        scope.Declare("argv", argsVal);
    }

    private static void InjectScriptPath(PilotScope scope, string? scriptPath)
    {
        // 2. Script path
        scope.Declare("script_path", PilotValue.FromString(scriptPath ?? string.Empty));
    }

    private static void InjectEnvironmentVariables(PilotScope scope)
    {
        // 3. Environment variables: env (shallow copy of cached template)
        var envMap = new Dictionary<PilotValue, PilotValue>(CachedEnvTemplate.Value);
        scope.Declare("env", PilotValue.FromMap(envMap));
    }

    private static void InjectProcessControlFunctions(PilotScope scope)
    {
        // 4. cwd(): Current working directory
        scope.Declare("cwd", CwdFunc);

        // 5. exit(code): Script termination
        scope.Declare("exit", ExitFunc);
    }

    private static void InjectFileSystemFunctions(PilotScope scope)
    {
        // 6. read_text(path): Reads entire file as text
        scope.Declare("read_text", ReadTextFunc);

        // 7. write_text(path, content): Writes text content to file
        scope.Declare("write_text", WriteTextFunc);

        // 8. file_exists(path): Checks if file exists
        scope.Declare("file_exists", FileExistsFunc);

        // 9. delete_file(path): Deletes a file if present
        scope.Declare("delete_file", DeleteFileFunc);
    }

    private static void InjectUtilityFunctions(PilotScope scope)
    {
        // 10. println(args...): Convenience println
        scope.Declare("println", PrintlnFunc);
    }
}
