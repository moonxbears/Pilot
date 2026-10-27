using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Compiler;
using Pilot.Diagnostics;
using Pilot.Preprocessing;
using Pilot.Scripting;
using Pilot.Testing;

namespace Pilot.Cli;

/// <summary>
/// Main CLI entry point for the Pilot language toolchain.
/// Supports both compilation and direct scripting execution.
/// </summary>
public static class Program
{
    public static int Main(string[] args)
    {
        Console.OutputEncoding = System.Text.Encoding.UTF8;

        if (args.Length > 0 && args[0] == "--all")
        {
            return PilotTestRunner.RunAll();
        }

        // 1. Direct inline script execution: pilot -c "code..." [args...]
        if (args.Length > 1 && (args[0] == "-c" || args[0] == "--command"))
        {
            string code = args[1];
            string[] scriptArgs = args.Length > 2 ? args.Skip(2).ToArray() : Array.Empty<string>();
            return ExecuteInlineScript(code, scriptArgs);
        }

        // 2. Compilation subcommand: pilot compile <file.pilot> [-o <out.pilc>]
        if (args.Length > 1 && (args[0] == "compile" || args[0] == "--compile"))
        {
            return ExecuteCompilationSubcommand(args.Skip(1).ToArray());
        }

        // 2b. Binary / Script Patch subcommand: pilot patch <base.pilc|.pilot> <patch.pilot|.pilc> [-o <out.pilc>]
        if (args.Length > 1 && (args[0] == "patch" || args[0] == "--patch-binary"))
        {
            return ExecutePatchSubcommand(args.Skip(1).ToArray());
        }

        // 2c. Live Hot-Reload Watch subcommand: pilot watch <file.pilot|.pilc> [args...]
        if (args.Length > 1 && (args[0] == "watch" || args[0] == "--watch"))
        {
            return ExecuteWatchSubcommand(args.Skip(1).ToArray());
        }

        // 2d. Native AOT Build subcommand: pilot build <file.pilot> [-o <out>] [--target <native|llvm|wasm|wat|cabi|c>]
        if (args.Length > 1 && (args[0] == "build" || args[0] == "--build"))
        {
            return ExecuteBuildSubcommand(args.Skip(1).ToArray());
        }

        // 3. Script run subcommand: pilot run <file.pilot|file.pilc> [--patch <patch.pilot|.pilc>] [args...]
        if (args.Length > 1 && (args[0] == "run"))
        {
            string scriptFile = args[1];
            string[] scriptArgs = args.Length > 2 ? args.Skip(2).ToArray() : Array.Empty<string>();
            return RunScriptFile(scriptFile, scriptArgs);
        }

        // 5. Interactive debugger subcommand: pilot debug <file.pilot> [args...]
        if (args.Length > 1 && (args[0] == "debug" || args[0] == "--debug"))
        {
            string scriptFile = args[1];
            string[] scriptArgs = args.Length > 2 ? args.Skip(2).ToArray() : Array.Empty<string>();
            return new Pilot.Debugging.PilotCliDebugger().Run(scriptFile, scriptArgs);
        }

        // 6. Universal Package Manager subcommands: pilot pkg <init|add|install|restore|remove|list|pack>
        if (args.Length > 0 && (args[0] == "pkg" || args[0] == "package" || args[0] == "install" || args[0] == "add" || args[0] == "restore" || args[0] == "pack"))
        {
            return ExecutePackageSubcommand(args);
        }

        // 7. Virtual Environment subcommand: pilot venv <create|info|activate> [path]
        if (args.Length > 0 && (args[0] == "venv" || args[0] == "env"))
        {
            return ExecuteVenvSubcommand(args);
        }

        // Auto-detect virtual environment in current or parent directory
        string? autoVenv = Pilot.Runtime.Venv.PilotVirtualEnvironment.Detect(Directory.GetCurrentDirectory());
        if (autoVenv != null && !Pilot.Runtime.Venv.PilotVirtualEnvironment.IsActive)
        {
            Pilot.Runtime.Venv.PilotVirtualEnvironment.Activate(autoVenv);
        }

        // 4. Direct script execution if first argument is a file: pilot script.pilot [args...] or pilot script.pilc [args...]
        bool hasToolingFlags = args.Any(a => a is "--emit" or "--target" or "--from" or "--ingest" or "--tokens" or "--tree");
        if (!hasToolingFlags && args.Length > 0 && !args[0].StartsWith("-") &&
            (args[0].EndsWith(".pilot", StringComparison.OrdinalIgnoreCase) || args[0].EndsWith(".pilc", StringComparison.OrdinalIgnoreCase)) &&
            File.Exists(args[0]))
        {
            string scriptFile = args[0];
            string[] scriptArgs = args.Length > 1 ? args.Skip(1).ToArray() : Array.Empty<string>();
            return RunScriptFile(scriptFile, scriptArgs);
        }

        bool showTokens = false;
        bool showTree = false;
        bool execute = false;
        bool startRepl = false;
        string? emitTarget = null;
        string? fromSource = null;
        string? targetFile = null;
        string? outputFile = null;
        var definedSymbols = new List<string>();
        var scriptArgsList = new List<string>();

        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--tokens") showTokens = true;
            else if (args[i] == "--tree") showTree = true;
            else if (args[i] == "--exec" || args[i] == "--run") execute = true;
            else if (args[i] == "--repl" || args[i] == "-i") startRepl = true;
            else if ((args[i] == "--emit" || args[i] == "--target") && i + 1 < args.Length)
            {
                emitTarget = args[++i];
            }
            else if ((args[i] == "--from" || args[i] == "--ingest") && i + 1 < args.Length)
            {
                fromSource = args[++i];
            }
            else if ((args[i] == "--define" || args[i] == "-D") && i + 1 < args.Length)
            {
                definedSymbols.Add(args[++i]);
            }
            else if ((args[i] == "-o" || args[i] == "--output") && i + 1 < args.Length)
            {
                outputFile = args[++i];
            }
            else if (!args[i].StartsWith("-"))
            {
                if (targetFile == null)
                {
                    targetFile = args[i];
                }
                else
                {
                    scriptArgsList.Add(args[i]);
                }
            }
        }

        if (startRepl)
        {
            return new PilotRepl().Run();
        }

        if (!string.IsNullOrEmpty(fromSource) && !string.IsNullOrEmpty(targetFile) && Directory.Exists(targetFile))
        {
            string outDir = outputFile ?? (targetFile.TrimEnd('/', '\\') + "_pilot");
            var libRes = Pilot.Transpilation.PilotLibraryTranspiler.IngestLibraryToPilot(targetFile, outDir, fromSource);
            Console.WriteLine($"[LIBRARY INGEST] Converted {libRes.GeneratedFiles.Count} files into '{outDir}'");
            return libRes.Success ? 0 : 1;
        }
        if (!string.IsNullOrEmpty(emitTarget) && !string.IsNullOrEmpty(targetFile) && Directory.Exists(targetFile))
        {
            string outDir = outputFile ?? (targetFile.TrimEnd('/', '\\') + "_" + emitTarget);
            var libRes = Pilot.Transpilation.PilotLibraryTranspiler.TranspilePilotLibraryToTarget(targetFile, outDir, emitTarget);
            Console.WriteLine($"[LIBRARY EMIT] Transpiled {libRes.GeneratedFiles.Count} files into '{outDir}'");
            return libRes.Success ? 0 : 1;
        }
        if (!string.IsNullOrEmpty(fromSource) && !string.IsNullOrEmpty(targetFile) && File.Exists(targetFile))
        {
            string foreignCode = File.ReadAllText(targetFile);
            string resolvedSourceLang = string.Equals(fromSource, "auto", StringComparison.OrdinalIgnoreCase)
                ? Pilot.Transpilation.TranspilationRegistry.InferLanguageFromPath(targetFile)
                : fromSource;
            var result = Pilot.Transpilation.TranspilationRegistry.Default.Ingest(foreignCode, resolvedSourceLang);
            if (!result.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[INGEST FAIL] {string.Join("; ", result.Diagnostics)}");
                Console.ResetColor();
                return 1;
            }
            if (!string.IsNullOrEmpty(outputFile)) { File.WriteAllText(outputFile, result.Code); Console.WriteLine($"[ output written to '{outputFile}' ]"); } else { Console.WriteLine(result.Code); }
            return 0;
        }

        if (string.IsNullOrEmpty(targetFile))
        {
            PrintUsage();
            targetFile = Path.Combine(Directory.GetCurrentDirectory(), "examples", "overview.pilot");
            if (!File.Exists(targetFile))
            {
                return 0;
            }
        }

        if (!File.Exists(Path.GetFullPath(targetFile)))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] File not found: '{targetFile}'");
            Console.ResetColor();
            return 1;
        }

        // Running a pre-compiled binary (.pilc)
        if (targetFile.EndsWith(".pilc", StringComparison.OrdinalIgnoreCase))
        {
            return RunScriptFile(targetFile, scriptArgsList.ToArray());
        }

        if (!string.IsNullOrEmpty(emitTarget))
        {
            string pilotCode = File.ReadAllText(targetFile);
            var result = Pilot.Transpilation.TranspilationRegistry.Default.Transpile(pilotCode, emitTarget);
            if (!result.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[TRANSPILATION FAIL] {string.Join("; ", result.Diagnostics)}");
                Console.ResetColor();
                return 1;
            }
            if (!string.IsNullOrEmpty(outputFile)) { File.WriteAllText(outputFile, result.Code); Console.WriteLine($"[ output written to '{outputFile}' ]"); } else { Console.WriteLine(result.Code); }
            return 0;
        }

        return ParseAndReport(targetFile, showTokens, showTree, definedSymbols, execute, scriptArgsList);
    }

    private static int ExecuteInlineScript(string code, string[] scriptArgs)
    {
        var result = PilotScript.Run(code, scriptArgs);
        if (!result.Success)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[SCRIPT ERROR] {string.Join("; ", result.Diagnostics)}");
            Console.ResetColor();
            return 1;
        }
        return 0;
    }

    private static int ExecuteBuildSubcommand(string[] args)
    {
        string? inputFile = null;
        string? outputFile = null;
        var target = Pilot.Native.NativeCompilationTarget.NativeExecutable;
        int optLevel = 2;
        bool targetWasi = true;

        for (int i = 0; i < args.Length; i++)
        {
            if ((args[i] == "-o" || args[i] == "--output") && i + 1 < args.Length)
            {
                outputFile = args[++i];
            }
            else if (args[i] == "--target" && i + 1 < args.Length)
            {
                string t = args[++i].ToLowerInvariant();
                target = t switch
                {
                    "llvm" or "ll" or "ir" => Pilot.Native.NativeCompilationTarget.LlvmIr,
                    "wasm" => Pilot.Native.NativeCompilationTarget.WebAssemblyBinary,
                    "wat" => Pilot.Native.NativeCompilationTarget.WebAssemblyText,
                    "cabi" or "header" or "h" => Pilot.Native.NativeCompilationTarget.CAbiHeader,
                    "c" => Pilot.Native.NativeCompilationTarget.CSource,
                    "cpp" or "c++" => Pilot.Native.NativeCompilationTarget.CppSource,
                    _ => Pilot.Native.NativeCompilationTarget.NativeExecutable
                };
            }
            else if (args[i] is "--emit-llvm" or "--emit-ir")
            {
                target = Pilot.Native.NativeCompilationTarget.LlvmIr;
            }
            else if (args[i] is "--emit-wasm")
            {
                target = Pilot.Native.NativeCompilationTarget.WebAssemblyBinary;
            }
            else if (args[i] is "--emit-wat")
            {
                target = Pilot.Native.NativeCompilationTarget.WebAssemblyText;
            }
            else if (args[i] is "--emit-lib" or "--emit-cabi" or "--emit-header")
            {
                target = Pilot.Native.NativeCompilationTarget.CAbiHeader;
            }
            else if (args[i] is "-O0") optLevel = 0;
            else if (args[i] is "-O1") optLevel = 1;
            else if (args[i] is "-O2") optLevel = 2;
            else if (args[i] is "-O3") optLevel = 3;
            else if (args[i] == "--opt-level" && i + 1 < args.Length && int.TryParse(args[++i], out int ol))
            {
                optLevel = ol;
            }
            else if (!args[i].StartsWith("-"))
            {
                inputFile = args[i];
            }
        }

        if (string.IsNullOrEmpty(inputFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine("[ERROR] Missing source file to build. Usage: pilot build <file.pilot> [-o <out>] [--target <native|llvm|wasm|wat|cabi|c>]");
            Console.ResetColor();
            return 1;
        }

        if (!File.Exists(inputFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] File not found: '{inputFile}'");
            Console.ResetColor();
            return 1;
        }

        Console.WriteLine($"--> Building Native AOT Target '{target}' for '{inputFile}'...");
        var options = new Pilot.Native.PilotNativeOptions
        {
            Target = target,
            OutputPath = outputFile,
            OptimizationLevel = optLevel,
            TargetWasi = targetWasi
        };

        var result = Pilot.Native.PilotNativeCompiler.CompileFile(inputFile, options);
        if (!result.Success)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[AOT BUILD FAILURE] Errors building '{inputFile}':");
            foreach (var diag in result.Diagnostics)
            {
                Console.WriteLine($"  - {diag}");
            }
            Console.ResetColor();
            return 1;
        }

        Console.ForegroundColor = ConsoleColor.Green;
        string artDesc = result.ArtifactPath ?? outputFile ?? target.ToString();
        Console.WriteLine($"[AOT BUILD SUCCESS] Generated target '{target}' -> '{artDesc}'");
        Console.ResetColor();

        foreach (var diag in result.Diagnostics)
        {
            Console.ForegroundColor = ConsoleColor.Yellow;
            Console.WriteLine($"  [NOTE] {diag}");
            Console.ResetColor();
        }

        return 0;
    }

    private static int ExecuteCompilationSubcommand(string[] args)
    {
        string? inputFile = null;
        string? outputFile = null;
        var symbols = new List<string>();

        for (int i = 0; i < args.Length; i++)
        {
            if ((args[i] == "-o" || args[i] == "--output") && i + 1 < args.Length)
            {
                outputFile = args[++i];
            }
            else if ((args[i] == "-D" || args[i] == "--define") && i + 1 < args.Length)
            {
                symbols.Add(args[++i]);
            }
            else if (!args[i].StartsWith("-"))
            {
                inputFile = args[i];
            }
        }

        if (string.IsNullOrEmpty(inputFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine("[ERROR] Missing source file to compile. Usage: pilot compile <file.pilot> [-o <out.pilc>]");
            Console.ResetColor();
            return 1;
        }

        if (!File.Exists(inputFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] File not found: '{inputFile}'");
            Console.ResetColor();
            return 1;
        }

        outputFile ??= Path.ChangeExtension(inputFile, ".pilc");

        Console.WriteLine($"--> Compiling '{inputFile}' -> '{outputFile}'...");
        var options = new PilotCompilerOptions(symbols)
            .WithSourceFile(inputFile);

        var result = PilotCompiler.Default.CompileFile(inputFile, options);
        if (!result.Success || result.Program == null)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[COMPILATION FAILED] ({result.Elapsed.TotalMilliseconds:F2}ms):");
            foreach (var diag in result.Diagnostics)
            {
                Console.WriteLine($"  {diag}");
            }
            Console.ResetColor();
            return 1;
        }

        result.Program.Save(outputFile);

        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine($"[COMPILATION SUCCESS] Generated '{outputFile}' ({new FileInfo(outputFile).Length} bytes) in {result.Elapsed.TotalMilliseconds:F2}ms");
        Console.ResetColor();
        return 0;
    }

    private static int RunScriptFile(string filePath, string[] scriptArgs)
    {
        if (!File.Exists(filePath))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] Script file not found: '{filePath}'");
            Console.ResetColor();
            return 1;
        }

        var patchFiles = new List<string>();
        var cleanArgs = new List<string>();
        for (int i = 0; i < scriptArgs.Length; i++)
        {
            if ((scriptArgs[i] == "--patch" || scriptArgs[i] == "-p") && i + 1 < scriptArgs.Length)
            {
                patchFiles.Add(scriptArgs[++i]);
            }
            else
            {
                cleanArgs.Add(scriptArgs[i]);
            }
        }

        if (patchFiles.Count > 0)
        {
            byte[] baseBytes = File.ReadAllBytes(filePath);
            PilotCompiledProgram baseProg;
            if (Pilot.Runtime.HotReload.PilotHotReloadSession.IsCompiledBinary(baseBytes, filePath))
            {
                baseProg = PilotCompiledProgram.FromBytes(baseBytes, filePath);
            }
            else
            {
                var comp = PilotCompiler.Default.CompileFile(filePath);
                if (!comp.Success || comp.Program is not PilotCompiledProgram compiledBase)
                {
                    Console.ForegroundColor = ConsoleColor.Red;
                    Console.WriteLine($"[COMPILATION ERROR] {string.Join("; ", comp.Diagnostics)}");
                    Console.ResetColor();
                    return 1;
                }
                baseProg = compiledBase;
            }

            foreach (var pf in patchFiles)
            {
                baseProg = baseProg.ApplyPatchFile(pf);
            }

            var patchedRes = baseProg.RunWithArgs(cleanArgs.ToArray());
            if (!patchedRes.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[PATCHED RUN ERROR] {string.Join("; ", patchedRes.Diagnostics)}");
                Console.ResetColor();
                return 1;
            }
            return 0;
        }

        var result = PilotScript.RunFile(filePath, cleanArgs.ToArray());
        if (!result.Success)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[SCRIPT RUN ERROR] ({result.Elapsed.TotalMilliseconds:F2}ms):");
            foreach (var diag in result.Diagnostics)
            {
                Console.WriteLine($"  {diag}");
            }
            Console.ResetColor();
            return 1;
        }

        return 0;
    }

    private static int ExecutePatchSubcommand(string[] args)
    {
        string? baseFile = null;
        var patchFiles = new List<string>();
        string? outputFile = null;

        for (int i = 0; i < args.Length; i++)
        {
            if ((args[i] == "-o" || args[i] == "--output") && i + 1 < args.Length)
            {
                outputFile = args[++i];
            }
            else if (!args[i].StartsWith("-"))
            {
                if (baseFile == null) baseFile = args[i];
                else patchFiles.Add(args[i]);
            }
        }

        if (string.IsNullOrEmpty(baseFile) || patchFiles.Count == 0)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine("[ERROR] Usage: pilot patch <base.pilc|base.pilot> <patch.pilot|patch.pilc> [-o <out.pilc|out.pilot>]");
            Console.ResetColor();
            return 1;
        }

        if (!File.Exists(baseFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] Base file not found: '{baseFile}'");
            Console.ResetColor();
            return 1;
        }

        byte[] baseBytes = File.ReadAllBytes(baseFile);
        PilotCompiledProgram currentProg;
        if (Pilot.Runtime.HotReload.PilotHotReloadSession.IsCompiledBinary(baseBytes, baseFile))
        {
            currentProg = PilotCompiledProgram.FromBytes(baseBytes, baseFile);
        }
        else
        {
            var comp = PilotCompiler.Default.CompileFile(baseFile);
            if (!comp.Success || comp.Program is not PilotCompiledProgram compiledBase)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[PATCH ERROR] Base compilation failed: {string.Join("; ", comp.Diagnostics)}");
                Console.ResetColor();
                return 1;
            }
            currentProg = compiledBase;
        }

        foreach (var pf in patchFiles)
        {
            if (!File.Exists(pf))
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[ERROR] Patch file not found: '{pf}'");
                Console.ResetColor();
                return 1;
            }
            currentProg = currentProg.ApplyPatchFile(pf);
        }

        outputFile ??= baseFile;
        if (outputFile.EndsWith(".pilot", StringComparison.OrdinalIgnoreCase))
        {
            File.WriteAllText(outputFile, currentProg.Source);
        }
        else
        {
            currentProg.Save(outputFile);
        }

        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine($"[PATCH SUCCESS] Applied {patchFiles.Count} patch(es) to '{baseFile}' -> '{outputFile}'");
        Console.ResetColor();
        return 0;
    }

    private static int ExecuteWatchSubcommand(string[] args)
    {
        if (args.Length == 0 || !File.Exists(args[0]))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine("[ERROR] Usage: pilot watch <file.pilot|file.pilc>");
            Console.ResetColor();
            return 1;
        }

        string filePath = Path.GetFullPath(args[0]);
        Console.ForegroundColor = ConsoleColor.Cyan;
        Console.WriteLine($"[HOT-RELOAD WATCH] Watching '{filePath}' for live patches (Ctrl+C to stop)...");
        Console.ResetColor();

        var engine = new Pilot.Runtime.Execution.PilotEngine();
        var execResult = File.ReadAllBytes(filePath) is var raw && Pilot.Runtime.HotReload.PilotHotReloadSession.IsCompiledBinary(raw, filePath)
            ? PilotCompiledProgram.FromBytes(raw, filePath).Run()
            : engine.ExecuteFile(filePath);

        if (execResult.Context != null)
        {
            execResult.Context.HotReload.WatchFile(
                filePath,
                Pilot.Runtime.Values.PilotFunction.CreateNative("on_watch_reload", (_, cbArgs) =>
                {
                    Console.ForegroundColor = ConsoleColor.Green;
                    Console.WriteLine($"[HOT-RELOAD] Applied patch revision v{execResult.Context.HotReload.Version} from '{Path.GetFileName(filePath)}'");
                    Console.ResetColor();
                    return Pilot.Runtime.Values.PilotValue.Null;
                }));

            using var exitEvent = new System.Threading.ManualResetEventSlim(false);
            Console.CancelKeyPress += (_, e) =>
            {
                e.Cancel = true;
                exitEvent.Set();
            };
            exitEvent.Wait();
            execResult.Context.HotReload.Dispose();
        }

        return execResult.Success ? 0 : 1;
    }

    private static int ExecutePackageSubcommand(string[] args)
    {
        string curDir = Directory.GetCurrentDirectory();
        var pm = Pilot.Runtime.Packaging.PilotPackageManager.Default;
        Action<string> log = msg => Console.WriteLine(msg);

        string cmd = args[0];
        string subCmd = args.Length > 1 ? args[1] : string.Empty;

        // e.g. pilot install [pkg] or pilot pkg install
        if (cmd == "install" || (cmd == "pkg" && subCmd == "install") || (cmd == "package" && subCmd == "install") ||
            cmd == "restore" || (cmd == "pkg" && subCmd == "restore") || (cmd == "package" && subCmd == "restore"))
        {
            string? pkg = (cmd == "install" && args.Length > 1) ? args[1]
                        : (cmd == "pkg" && args.Length > 2) ? args[2] : null;

            if (!string.IsNullOrEmpty(pkg))
            {
                bool addOk = pm.Add(curDir, pkg, null, log);
                return addOk ? 0 : 1;
            }
            bool ok = pm.Restore(curDir, log);
            return ok ? 0 : 1;
        }

        // e.g. pilot add <pkg> or pilot pkg add <pkg>
        if (cmd == "add" || ((cmd == "pkg" || cmd == "package") && subCmd == "add"))
        {
            int pkgIdx = cmd == "add" ? 1 : 2;
            if (pkgIdx >= args.Length)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine("[PKG ERROR] Missing package to add. Usage: pilot pkg add <packageSpec>");
                Console.ResetColor();
                return 1;
            }
            string pkgSpec = args[pkgIdx];
            bool ok = pm.Add(curDir, pkgSpec, null, log);
            return ok ? 0 : 1;
        }

        // e.g. pilot pkg remove <pkg>
        if ((cmd == "pkg" || cmd == "package") && subCmd == "remove")
        {
            if (args.Length < 3)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine("[PKG ERROR] Missing package to remove. Usage: pilot pkg remove <packageSpec>");
                Console.ResetColor();
                return 1;
            }
            bool ok = pm.Remove(curDir, args[2], log);
            return ok ? 0 : 1;
        }

        // e.g. pilot pkg list
        if ((cmd == "pkg" || cmd == "package") && subCmd == "list")
        {
            pm.List(curDir, log);
            return 0;
        }

        // e.g. pilot pkg init [name]
        if ((cmd == "pkg" || cmd == "package") && subCmd == "init")
        {
            string? name = args.Length > 2 ? args[2] : null;
            var manifest = pm.Init(curDir, name);
            Console.ForegroundColor = ConsoleColor.Green;
            Console.WriteLine($"[PKG SUCCESS] Initialized Pilot package '{manifest.Name}' in '{curDir}/pilot.json'");
            Console.ResetColor();
            return 0;
        }

        // e.g. pilot pack or pilot pkg pack
        if (cmd == "pack" || ((cmd == "pkg" || cmd == "package") && subCmd == "pack"))
        {
            string? outPath = (cmd == "pack" && args.Length > 1) ? args[1]
                            : (args.Length > 2) ? args[2] : null;
            pm.Pack(curDir, outPath, log);
            return 0;
        }

        Console.ForegroundColor = ConsoleColor.Yellow;
        Console.WriteLine($"Unknown package command. Supported:");
        Console.WriteLine("  pilot pkg init [name]               # Initialize new package manifest (pilot.json)");
        Console.WriteLine("  pilot pkg add <packageSpec>         # Add native or interop dependency (e.g. nuget:Newtonsoft.Json, npm:lodash)");
        Console.WriteLine("  pilot pkg install / restore         # Restore all declared native and adopted dependencies");
        Console.WriteLine("  pilot pkg remove <packageSpec>      # Remove dependency");
        Console.WriteLine("  pilot pkg list                      # List all dependencies");
        Console.WriteLine("  pilot pkg pack [out.pilpkg]         # Bundle package into distributable .pilpkg");
        Console.ResetColor();
        return 1;
    }

    private static int ExecuteVenvSubcommand(string[] args)
    {
        string subCmd = args.Length > 1 ? args[1] : string.Empty;

        // pilot venv info [dir]
        if (subCmd == "info")
        {
            string targetDir = args.Length > 2 ? args[2] : (Pilot.Runtime.Venv.PilotVirtualEnvironment.ActiveVenvPath ?? Directory.GetCurrentDirectory());
            string? venvPath = Pilot.Runtime.Venv.PilotVirtualEnvironment.Detect(targetDir);
            if (venvPath == null)
            {
                Console.WriteLine("No active or detected Pilot virtual environment.");
                return 1;
            }

            var cfg = Pilot.Runtime.Venv.PilotVenvConfig.Load(Path.Combine(venvPath, Pilot.Runtime.Venv.PilotVirtualEnvironment.ConfigFileName));
            Console.WriteLine($"Pilot Virtual Environment: {venvPath}");
            Console.WriteLine($"  Prompt: {cfg.Prompt}");
            Console.WriteLine($"  Home: {cfg.Home}");
            Console.WriteLine($"  Version: {cfg.Version}");
            Console.WriteLine($"  Include System Packages: {cfg.IncludeSystemPackages}");
            Console.WriteLine($"  Created At: {cfg.CreatedAt:O}");
            Console.WriteLine($"  Is Active: {Pilot.Runtime.Venv.PilotVirtualEnvironment.ActiveVenvPath == venvPath}");
            return 0;
        }

        // pilot venv create <targetDir> [--prompt <name>] or pilot venv <targetDir>
        string venvTarget = (subCmd == "create" && args.Length > 2) ? args[2]
                          : (!string.IsNullOrEmpty(subCmd) && !subCmd.StartsWith("-")) ? subCmd
                          : ".pilotvenv";

        string? promptName = null;
        for (int i = 1; i < args.Length; i++)
        {
            if ((args[i] == "--prompt" || args[i] == "-p") && i + 1 < args.Length)
            {
                promptName = args[++i];
            }
        }

        Console.WriteLine($"--> Creating Pilot virtual environment in '{venvTarget}'...");
        var options = new Pilot.Runtime.Venv.PilotVenvOptions
        {
            Prompt = promptName
        };

        var config = Pilot.Runtime.Venv.PilotVirtualEnvironment.Create(venvTarget, options);
        string absPath = Path.GetFullPath(venvTarget);

        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine($"[VENV SUCCESS] Created Pilot virtual environment at '{absPath}'.");
        Console.ResetColor();

        string scriptsDir = OperatingSystem.IsWindows() ? Path.Combine(absPath, "Scripts") : Path.Combine(absPath, "bin");
        Console.WriteLine("\nTo activate this virtual environment:");
        if (OperatingSystem.IsWindows())
        {
            Console.WriteLine($"  PowerShell : & \"{Path.Combine(scriptsDir, "activate.ps1")}\"");
            Console.WriteLine($"  CMD        : \"{Path.Combine(scriptsDir, "activate.bat")}\"");
        }
        else
        {
            Console.WriteLine($"  Bash / Zsh : source \"{Path.Combine(scriptsDir, "activate")}\"");
        }
        return 0;
    }

    public static void PrintUsage()
    {
        Console.WriteLine("==================================================================");
        Console.WriteLine("                 PILOT LANGUAGE RUNNER & VALIDATOR                ");
        Console.WriteLine("==================================================================");
        Console.WriteLine("Scripting & Execution Usage:");
        Console.WriteLine("  pilot <script.pilot> [args...]       # Run script file directly");
        Console.WriteLine("  pilot <script.pilc> [args...]        # Run precompiled binary directly");
        Console.WriteLine("  pilot run <file.pilot> [args...]     # Explicitly run script");
        Console.WriteLine("  pilot debug <file.pilot> [args...]   # Start interactive CLI debugger session");
        Console.WriteLine("  pilot -c \"<code>\" [args...]         # Execute inline code string");
        Console.WriteLine("  pilot compile <file> [-o <out.pilc>] # Precompile to binary format");
        Console.WriteLine("\nVirtual Environment Usage (Isolation & Sandboxing):");
        Console.WriteLine("  pilot venv [path]                    # Create new virtual environment (default: .pilotvenv)");
        Console.WriteLine("  pilot venv create <path> [-p <name>] # Create named virtual environment");
        Console.WriteLine("  pilot venv info                      # Display active or local virtual environment metadata");
        Console.WriteLine("\nPackage Management Usage (Native & Adopted Interop):");
        Console.WriteLine("  pilot pkg init [name]                # Create new pilot.json manifest");
        Console.WriteLine("  pilot pkg add <pkg>                  # Add native or interop dependency");
        Console.WriteLine("                                       # (nuget:Pkg@Ver, npm:Pkg@Ver, pip:Pkg@Ver, cargo:Pkg@Ver)");
        Console.WriteLine("  pilot pkg install                    # Restore and install all dependencies");
        Console.WriteLine("  pilot pkg remove <pkg>               # Remove dependency from project");
        Console.WriteLine("  pilot pkg list                       # List all dependencies");
        Console.WriteLine("  pilot pkg pack [out.pilpkg]          # Package project into distributable archive");
        Console.WriteLine("\nTooling & Analysis Usage:");
        Console.WriteLine("  pilot --tokens <file>                # Dump lexical token stream");
        Console.WriteLine("  pilot --tree <file>                  # Print hierarchical AST");
        Console.WriteLine("  pilot --repl                         # Start interactive REPL");
        Console.WriteLine("  pilot --emit <target> <file>         # Transpile to target (python, csharp, rust)");
        Console.WriteLine("  pilot --from <lang> <file>           # Ingest foreign code to Pilot");
        Console.WriteLine("  pilot -D <symbol> <file>             # Predefine compilation symbol");
        Console.WriteLine("  pilot --all                          # Run all test scripts in tests/");
        Console.WriteLine("==================================================================\n");
    }

    public static int ParseAndReport(
        string filePath,
        bool showTokens,
        bool showTree,
        IEnumerable<string>? definedSymbols = null,
        bool execute = false,
        IReadOnlyList<string>? scriptArgs = null)
    {
        Console.WriteLine($"\n--> Parsing: {filePath}");
        string source = File.ReadAllText(filePath);

        var preproc = PilotPreprocessor.Preprocess(
            source,
            filePath,
            new PilotPreprocessorOptions(definedSymbols ?? Array.Empty<string>()));

        foreach (var warn in preproc.Diagnostics.Where(d => d.Severity == PreprocessorDiagnosticSeverity.Warning))
        {
            Console.ForegroundColor = ConsoleColor.Yellow;
            Console.WriteLine($"  [PREPROC WARNING] {warn.Message} (line {warn.Line})");
            Console.ResetColor();
        }

        if (preproc.HasErrors)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"\n[FAIL] Found {preproc.Diagnostics.Count(d => d.Severity == PreprocessorDiagnosticSeverity.Error)} preprocessor error(s):");
            foreach (var err in preproc.Diagnostics.Where(d => d.Severity == PreprocessorDiagnosticSeverity.Error))
            {
                Console.WriteLine($"  [{err.Line}:{err.Column}] {err.Message}");
            }
            Console.ResetColor();
            return 1;
        }

        var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);

        if (showTokens)
        {
            Console.WriteLine("\n--- Token Stream ---");
            var tokens = lexer.GetAllTokens();
            foreach (var t in tokens)
            {
                string name = PilotLexer.DefaultVocabulary.GetSymbolicName(t.Type);
                string text = t.Text.Replace("\r", "\\r").Replace("\n", "\\n");
                Console.WriteLine($"  Line {t.Line,3}:{t.Column,-3} Token: {name,-22} Text: '{text}' (Channel: {t.Channel})");
            }
            lexer.Reset();
        }

        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);

        var errorListener = new ErrorTracker();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorListener);

        var tree = parser.ParseProgramFast();

        if (errorListener.HasErrors)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"\n[FAIL] Found {errorListener.Errors.Count} syntax error(s):");
            foreach (var err in errorListener.Errors)
            {
                Console.WriteLine($"  {err}");
            }
            Console.ResetColor();
            return 1;
        }

        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine($"[SUCCESS] Parsed successfully with 0 errors!");
        Console.ResetColor();
        Console.WriteLine($"  Root Context: {tree.GetType().Name}");
        Console.WriteLine($"  Top-level nodes: {tree.ChildCount}");

        if (showTree)
        {
            Console.WriteLine("\n--- Hierarchical Parse Tree ---");
            ParseTreeVisualizer.Print(tree, parser, 0);
        }

        if (execute)
        {
            Console.WriteLine("\n--> Executing with PilotCompiledProgram...");
            var program = new PilotCompiledProgram(preproc.PreprocessedSource, tree, filePath, DateTime.UtcNow, definedSymbols);
            var result = program.RunWithArgs(scriptArgs ?? Array.Empty<string>());
            if (!result.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"\n[RUNTIME FAIL] Execution failed ({result.Elapsed.TotalMilliseconds:F2}ms):");
                foreach (var err in result.Diagnostics)
                {
                    Console.WriteLine($"  {err}");
                }
                Console.ResetColor();
                return 1;
            }

            Console.ForegroundColor = ConsoleColor.Green;
            Console.WriteLine($"\n[RUNTIME SUCCESS] Executed in {result.Elapsed.TotalMilliseconds:F2}ms");
            Console.ResetColor();
            Console.WriteLine($"  Final Value: {result.Value}");
        }

        return 0;
    }
}
