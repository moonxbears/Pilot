using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using Pilot.Native.CAbi;
using Pilot.Native.Llvm;
using Pilot.Native.Wasm;
using Pilot.Transpilation;

namespace Pilot.Native;

/// <summary>
/// Target platform or representation for Native Ahead-Of-Time (AOT) compilation.
/// </summary>
public enum NativeCompilationTarget
{
    NativeExecutable,
    LlvmIr,
    WebAssemblyBinary,
    WebAssemblyText,
    CAbiHeader,
    CSource,
    CppSource,
    Assembly
}

/// <summary>
/// Configuration options for Native Ahead-Of-Time (AOT) compilation.
/// </summary>
public sealed class PilotNativeOptions
{
    public NativeCompilationTarget Target { get; set; } = NativeCompilationTarget.NativeExecutable;
    public string? OutputPath { get; set; }
    public int OptimizationLevel { get; set; } = 2;
    public bool TargetWasi { get; set; } = true;
    public bool EmitHeaderComments { get; set; } = true;
    public string ModuleName { get; set; } = "pilot_module";
    public string? TargetTriple { get; set; }
    public bool BurstOptimized { get; set; } = true;
}

/// <summary>
/// Result of an AOT compilation process.
/// </summary>
public sealed class PilotNativeResult
{
    public bool Success { get; init; }
    public NativeCompilationTarget Target { get; init; }
    public string? ArtifactPath { get; init; }
    public string? OutputCode { get; init; }
    public byte[]? BinaryBytes { get; init; }
    public List<string> Diagnostics { get; } = new();

    public static PilotNativeResult CreateSuccess(
        NativeCompilationTarget target,
        string? artifactPath,
        string? outputCode = null,
        byte[]? binaryBytes = null)
    {
        return new PilotNativeResult
        {
            Success = true,
            Target = target,
            ArtifactPath = artifactPath,
            OutputCode = outputCode,
            BinaryBytes = binaryBytes
        };
    }

    public static PilotNativeResult CreateFailure(
        NativeCompilationTarget target,
        IEnumerable<string> diagnostics)
    {
        var res = new PilotNativeResult
        {
            Success = false,
            Target = target
        };
        res.Diagnostics.AddRange(diagnostics);
        return res;
    }
}

/// <summary>
/// Orchestrates Native Ahead-Of-Time (AOT) compilation across LLVM IR, WebAssembly, and C-ABI targets.
/// Probes for system toolchains (Clang, LLC, GCC) and produces optimized platform executables,
/// LLVM IR modules (.ll), WebAssembly binaries (.wasm), and C headers (.h).
/// </summary>
public static class PilotNativeCompiler
{
    /// <summary>
    /// Compiles Pilot source code into the specified native target.
    /// </summary>
    public static PilotNativeResult Compile(string source, PilotNativeOptions? options = null)
    {
        options ??= new PilotNativeOptions();
        string modName = options.ModuleName;

        try
        {
            switch (options.Target)
            {
                case NativeCompilationTarget.Assembly:
                {
                    string asm = Pilot.Native.Assembly.PilotAssemblyEmitter.EmitAssembly(
                        source,
                        modName,
                        options.OptimizationLevel,
                        options.BurstOptimized);
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllText(outPath, asm);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, asm);
                }

                case NativeCompilationTarget.LlvmIr:
                {
                    string llvmIr = PilotLlvmEmitter.Emit(source, modName);
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllText(outPath, llvmIr);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, llvmIr);
                }

                case NativeCompilationTarget.WebAssemblyText:
                {
                    string wat = PilotWasmEmitter.EmitWat(source, options.TargetWasi);
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllText(outPath, wat);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, wat);
                }

                case NativeCompilationTarget.WebAssemblyBinary:
                {
                    byte[] wasmBytes = PilotWasmEmitter.EmitWasm(source, options.TargetWasi);
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllBytes(outPath, wasmBytes);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, null, wasmBytes);
                }

                case NativeCompilationTarget.CAbiHeader:
                {
                    string header = PilotCAbiEmitter.EmitHeader(source, modName);
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllText(outPath, header);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, header);
                }

                case NativeCompilationTarget.CSource:
                {
                    var transRes = TranspilationRegistry.Default.Transpile(source, "c");
                    if (!transRes.Success)
                    {
                        return PilotNativeResult.CreateFailure(options.Target, transRes.Diagnostics);
                    }
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllText(outPath, transRes.Code);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, transRes.Code);
                }

                case NativeCompilationTarget.CppSource:
                {
                    var transRes = TranspilationRegistry.Default.Transpile(source, "cpp");
                    if (!transRes.Success)
                    {
                        return PilotNativeResult.CreateFailure(options.Target, transRes.Diagnostics);
                    }
                    string? outPath = options.OutputPath;
                    if (!string.IsNullOrEmpty(outPath))
                    {
                        File.WriteAllText(outPath, transRes.Code);
                    }
                    return PilotNativeResult.CreateSuccess(options.Target, outPath, transRes.Code);
                }

                case NativeCompilationTarget.NativeExecutable:
                default:
                {
                    return BuildNativeExecutable(source, options);
                }
            }
        }
        catch (Exception ex)
        {
            return PilotNativeResult.CreateFailure(options.Target, new[] { $"Native compilation error: {ex.Message}" });
        }
    }

    /// <summary>
    /// Compiles a Pilot source file on disk into the specified native target.
    /// </summary>
    public static PilotNativeResult CompileFile(string filePath, PilotNativeOptions? options = null)
    {
        if (!File.Exists(filePath))
        {
            return PilotNativeResult.CreateFailure(
                options?.Target ?? NativeCompilationTarget.NativeExecutable,
                new[] { $"Source file not found: '{filePath}'" });
        }

        options ??= new PilotNativeOptions();
        if (string.IsNullOrEmpty(options.ModuleName) || options.ModuleName == "pilot_module")
        {
            options.ModuleName = Path.GetFileNameWithoutExtension(filePath);
        }

        string source = File.ReadAllText(filePath);
        return Compile(source, options);
    }

    /// <summary>
    /// Builds a native executable binary by compiling to LLVM IR and invoking the system toolchain if present.
    /// </summary>
    private static PilotNativeResult BuildNativeExecutable(string source, PilotNativeOptions options)
    {
        string llvmIr = PilotLlvmEmitter.Emit(source, options.ModuleName);

        string tempDir = Path.Combine(Path.GetTempPath(), $"pilot_aot_{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempDir);
        string llPath = Path.Combine(tempDir, $"{options.ModuleName}.ll");
        File.WriteAllText(llPath, llvmIr);

        string exeExt = OperatingSystem.IsWindows() ? ".exe" : "";
        string outExe = options.OutputPath ?? Path.Combine(Directory.GetCurrentDirectory(), $"{options.ModuleName}{exeExt}");

        string? compilerTool = ProbeCompilerToolchain();
        if (compilerTool == null)
        {
            // Toolchain not found on host machine: preserve generated LLVM IR file cleanly
            string fallbackLlPath = Path.ChangeExtension(outExe, ".ll");
            File.Copy(llPath, fallbackLlPath, overwrite: true);
            try { Directory.Delete(tempDir, true); } catch { }

            var res = PilotNativeResult.CreateSuccess(NativeCompilationTarget.LlvmIr, fallbackLlPath, llvmIr);
            res.Diagnostics.Add(
                $"Note: System C/LLVM toolchain (clang/gcc) was not found on PATH. Generated optimized LLVM IR to '{fallbackLlPath}'. " +
                "To produce a standalone native binary, install Clang or GCC and run 'clang " + fallbackLlPath + " -o " + outExe + "'.");
            return res;
        }

        // Run clang/gcc to compile LLVM IR into a native executable
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = compilerTool,
                Arguments = $"-O{options.OptimizationLevel} \"{llPath}\" -o \"{outExe}\"",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            using var proc = Process.Start(psi);
            if (proc != null)
            {
                string stderr = proc.StandardError.ReadToEnd();
                proc.WaitForExit(30000);

                if (proc.ExitCode == 0 && File.Exists(outExe))
                {
                    try { Directory.Delete(tempDir, true); } catch { }
                    return PilotNativeResult.CreateSuccess(NativeCompilationTarget.NativeExecutable, outExe);
                }
                else
                {
                    // Fall back to C transpilation if LLVM IR had strict ABI mismatch
                    return CompileViaCToolchain(source, options, compilerTool, outExe, tempDir, stderr);
                }
            }
        }
        catch (Exception ex)
        {
            // If invocation fails, fall back to LLVM IR output
            string fallbackLl = Path.ChangeExtension(outExe, ".ll");
            File.Copy(llPath, fallbackLl, overwrite: true);
            var res = PilotNativeResult.CreateSuccess(NativeCompilationTarget.LlvmIr, fallbackLl, llvmIr);
            res.Diagnostics.Add($"Toolchain execution encountered: {ex.Message}. LLVM IR emitted to '{fallbackLl}'.");
            return res;
        }

        return PilotNativeResult.CreateSuccess(NativeCompilationTarget.LlvmIr, llPath, llvmIr);
    }

    private static PilotNativeResult CompileViaCToolchain(
        string source,
        PilotNativeOptions options,
        string compilerTool,
        string outExe,
        string tempDir,
        string previousStderr)
    {
        var transRes = TranspilationRegistry.Default.Transpile(source, "c");
        if (!transRes.Success)
        {
            return PilotNativeResult.CreateFailure(NativeCompilationTarget.NativeExecutable,
                new[] { $"Toolchain failed on LLVM IR ({previousStderr}) and C fallback failed." });
        }

        string cPath = Path.Combine(tempDir, $"{options.ModuleName}.c");
        File.WriteAllText(cPath, transRes.Code);

        var psi = new ProcessStartInfo
        {
            FileName = compilerTool,
            Arguments = $"-O{options.OptimizationLevel} \"{cPath}\" -o \"{outExe}\"",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        using var proc = Process.Start(psi);
        if (proc != null)
        {
            string err = proc.StandardError.ReadToEnd();
            proc.WaitForExit(30000);
            if (proc.ExitCode == 0 && File.Exists(outExe))
            {
                try { Directory.Delete(tempDir, true); } catch { }
                return PilotNativeResult.CreateSuccess(NativeCompilationTarget.NativeExecutable, outExe);
            }
        }

        return PilotNativeResult.CreateFailure(NativeCompilationTarget.NativeExecutable,
            new[] { $"Toolchain compilation failed: {previousStderr}" });
    }

    private static string? ProbeCompilerToolchain()
    {
        string[] candidates = OperatingSystem.IsWindows()
            ? new[] { "clang", "gcc", "clang.exe", "gcc.exe" }
            : new[] { "clang", "gcc", "cc" };

        foreach (var c in candidates)
        {
            if (IsCommandAvailable(c))
            {
                return c;
            }
        }
        return null;
    }

    private static bool IsCommandAvailable(string command)
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = command,
                Arguments = "--version",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var proc = Process.Start(psi);
            if (proc != null)
            {
                proc.WaitForExit(2000);
                return proc.ExitCode == 0;
            }
        }
        catch
        {
            // Command does not exist
        }
        return false;
    }
}
