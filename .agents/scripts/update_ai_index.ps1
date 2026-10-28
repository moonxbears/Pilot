#Requires -Version 7.0
<#
.SYNOPSIS
    Generates and updates .agents/AI_CODEBASE_INDEX.md (AI-only fast navigation & symbol/line index).
    Preserves any content inside <!-- BEGIN MANUAL NOTES --> ... <!-- END MANUAL NOTES --> while
    regenerating live file line counts, C#/TS type declarations, grammar rule line anchors, and
    internal method maps for large core files (> 350 lines).
#>
param(
    [string]$RepoRoot = "",
    [switch]$CheckStampOnly
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
} else {
    $RepoRoot = (Resolve-Path $RepoRoot).Path
}

$IndexPath = Join-Path $RepoRoot ".agents\AI_CODEBASE_INDEX.md"
$StampPath = Join-Path $RepoRoot ".agents\.ai_index_stamp"

# Fast directory pruning (never descends into bin, obj, .git, node_modules, etc.; preserves Runtime/Venv!)
$SkipDirNames = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($d in @('bin', 'Bin', 'obj', 'Obj', '.git', '.vs', '.vscode', '.idea', 'TestResults', '.pilot_modules', 'node_modules', '__pycache__', '.antlr', 'nupkg', '.pilotvenv')) {
    [void]$SkipDirNames.Add($d)
}
$SkipExt = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
foreach ($e in @('.dll', '.exe', '.pdb', '.vsix', '.nupkg', '.snupkg', '.cache', '.user', '.suo', '.pilc', '.pilpkg', '.png', '.ico', '.map')) {
    [void]$SkipExt.Add($e)
}

$FileList = [System.Collections.Generic.List[PSCustomObject]]::new(512)
$DirQueue = [System.Collections.Generic.Queue[string]]::new()
$DirQueue.Enqueue($RepoRoot)

while ($DirQueue.Count -gt 0) {
    $currentDir = $DirQueue.Dequeue()
    foreach ($subDir in [System.IO.Directory]::GetDirectories($currentDir)) {
        $dirName = [System.IO.Path]::GetFileName($subDir)
        if ($SkipDirNames.Contains($dirName)) { continue }
        $relDir = [System.IO.Path]::GetRelativePath($RepoRoot, $subDir).Replace('\', '/')
        if ($relDir -ceq 'venv' -or $relDir -ceq '.venv' -or $relDir -ceq 'vscode-pilot/out' -or $relDir -ceq 'vscode-pilot/dist' -or $relDir -ceq 'vscode-pilot/server') {
            continue
        }
        $DirQueue.Enqueue($subDir)
    }
    foreach ($filePath in [System.IO.Directory]::GetFiles($currentDir)) {
        $ext = [System.IO.Path]::GetExtension($filePath)
        if ($SkipExt.Contains($ext)) { continue }
        $relFile = [System.IO.Path]::GetRelativePath($RepoRoot, $filePath).Replace('\', '/')
        if ($relFile -ceq '.agents/.ai_index_stamp' -or $relFile -ceq '.agents/AI_CODEBASE_INDEX.md') { continue }
        $fi = [System.IO.FileInfo]::new($filePath)
        $FileList.Add([PSCustomObject]@{
            FullPath  = $filePath
            RelPath   = $relFile
            Ext       = $ext.ToLowerInvariant()
            Length    = $fi.Length
            LastWrite = $fi.LastWriteTimeUtc.Ticks
        })
    }
}

$AllFiles = $FileList | Sort-Object RelPath

# Compute deterministic stamp hash
$StampSource = ($AllFiles | ForEach-Object { "$($_.RelPath)|$($_.Length)|$($_.LastWrite)" }) -join "`n"
$Sha = [System.Security.Cryptography.SHA256]::Create()
$StampBytes = $Sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($StampSource))
$StampHash = [BitConverter]::ToString($StampBytes).Replace('-', '')

if ($CheckStampOnly -and (Test-Path $IndexPath) -and (Test-Path $StampPath)) {
    $ExistingStamp = ([System.IO.File]::ReadAllText($StampPath)).Trim()
    if ($ExistingStamp -ceq $StampHash) {
        return
    }
}

function Format-WrappedItems([System.Collections.Generic.List[string]]$Items, [int]$MaxLineWidth = 115) {
    if ($Items.Count -eq 0) { return "" }
    $outSb = [System.Text.StringBuilder]::new()
    $currentLen = 0
    for ($k = 0; $k -lt $Items.Count; $k++) {
        $item = $Items[$k]
        if ($currentLen -eq 0) {
            [void]$outSb.Append($item)
            $currentLen = $item.Length
        }
        elseif (($currentLen + 3 + $item.Length) -le $MaxLineWidth) {
            [void]$outSb.Append(' · ').Append($item)
            $currentLen += 3 + $item.Length
        }
        else {
            [void]$outSb.AppendLine()
            [void]$outSb.Append($item)
            $currentLen = $item.Length
        }
    }
    return $outSb.ToString()
}

$DefaultManualNotes = @'
<!-- BEGIN MANUAL NOTES -->
## 1. Instant Task-to-File Routing Table (Consult First — Zero Blind Searching)

| Subsystem / Task | Primary Entry Files | Key Notes |
| :--- | :--- | :--- |
| **Roadmap, Concurrency & Invariants** | `MILESTONES.md` | Single source of truth for M1–M27 status, parallel waves, and grammar invariants. |
| **Master Lexer & Parser Grammar** | `antlr/Pilot.g4`, `Compiler/PilotParserFast.cs` | ANTLR4 combined grammar (generated C# parser goes to `obj/` on `dotnet build`). |
| **Preprocessor (`#define`, `#if`, `#pragma`)** | `Preprocessing/PilotPreprocessor.cs` | Runs before lexing; emits `PREPROC001`–`PREPROC011`. |
| **Tree-Walk AST Interpreter** | `Runtime/Execution/PilotInterpreterVisitor.cs`, `Runtime/Execution/PilotEngine.cs` | Evaluates statements/expressions in AST mode; pattern matching in `PilotPatternMatcher.cs`. |
| **Runtime Values & Primitives** | `Runtime/Values/PilotValue.cs`, `PilotValueKind.cs`, `PilotAdt.cs`, `PilotObject.cs` | Also `PilotFunction.cs`, `PilotMethodGroup.cs`, `PilotKeyword.cs`, `PilotMatrix.cs`, `PilotRange.cs`, `PilotGenerator.cs`, `PilotLazy.cs`. |
| **Bytecode Compiler & `.pilc` Format** | `Compiler/PilotBytecodeCompiler.cs`, `Compiler/PilotCompiledProgram.cs`, `Runtime/Bytecode/PilotOpcode.cs` | Two-phase AST -> PilVM chunk compiler and `.pilc` v2 serializer. |
| **PilVM Virtual Machine** | `Runtime/Bytecode/PilVirtualMachine.cs`, `Runtime/Bytecode/PilotBytecodeChunk.cs` | Stack/register bytecode VM execution loop. |
| **Static Analysis & Type Inference** | `Analysis/PilotAnalysisPipeline.cs`, `Analysis/TypeSystem/PilotTypeInferenceEngine.cs`, `PilotType.cs` | Orchestrates inference, exhaustiveness, borrow checking, dimensions, and contracts. |
| **Exhaustive Pattern Matching** | `Analysis/PatternMatching/PilotExhaustivenessChecker.cs`, `Runtime/Execution/PilotPatternMatcher.cs` | Compile-time usefulness/exhaustiveness checking + runtime destructuring. |
| **Ownership, Units & Contracts** | `Analysis/Ownership/PilotBorrowChecker.cs`, `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs`, `Analysis/ContractVerification/PilotContractVerifier.cs` | Runtime contracts registry in `Runtime/Contracts/PilotContractRegistry.cs`. |
| **Built-in Functions & Host Scripting** | `Runtime/Builtins/PilotBuiltinRegistry.cs`, `Scripting/PilotScriptContext.cs`, `Scripting/PilotScript.cs` | Global intrinsics (`println`, `len`, `json`, `venv`, `matrix`, file/OS helpers). |
| **Type Adapters & Extensions** | `Runtime/Adapters/PilotTypeAdapterRegistry.cs`, `Runtime/Extensions/PilotLexicalExtension.cs` | First-class `adapter` conversion chains & `extension ... on ...` dispatch. |
| **Compile-Time `@` Code Generators** | `CodeGen/Engine/PilotCodeGeneratorPipeline.cs`, `CodeGen/Builtins/*.cs` | `@builder`, `@equality`, `@json`, `@metadata`, `@observable`, `@repr`. |
| **Bidirectional Source Transpilers** | `Transpilation/TranspilationRegistry.cs`, `PilotLibraryTranspiler.cs`, `PilotLibraryMapper.cs`, `Transpilation/Targets/*` | `--emit <target>` and `--from <lang>` for C, C++, C#, Rust, Python, JS, Lua + `PilotOptimizer.cs`. |
| **Native AOT (LLVM, WASM, C-ABI, ASM)** | `Native/PilotNativeCompiler.cs`, `Native/Llvm/PilotLlvmEmitter.cs`, `Native/Wasm/PilotWasmEmitter.cs`, `Native/CAbi/PilotCAbiEmitter.cs`, `Native/Assembly/PilotAssemblyEmitter.cs` | Native C header in `Native/Runtime/pilot_runtime.h`; Burst/Zero-GC in `Runtime/Burst/PilotBurstEngine.cs`, `Runtime/NativeMemory/`. |
| **Polyglot Interop Blocks** | `Runtime/Interop/PolyglotRegistry.cs`, `Runtime/Interop/PolyglotTypeBridge.cs` | Inline Python, JS, Java, Lua, C, C++, ASM, C#, Rust, PostgreSQL, MySQL + yield bridges. |
| **Packages (`pkg`) & Virtual Envs (`venv`)** | `Runtime/Packaging/PilotPackageManager.cs`, `Runtime/Packaging/Adapters/BaseInteropPackageAdapter.cs`, `Runtime/Venv/PilotVirtualEnvironment.cs` | `.pilpkg`, `pilot.json`, `pilot.lock.json`, foreign package adoption, `pilotvenv.cfg`. |
| **Debugger & Hot Reload** | `Debugging/PilotDebugger.cs`, `Debugging/PilotCliDebugger.cs`, `Runtime/HotReload/PilotHotReloader.cs` | Breakpoints, stepping, call stack, state-preserving hot reload. |
| **LSP Server & Semantic Diagnostics** | `Pilot.LanguageServer/Lsp/PilotLspServer.cs`, `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs`, `PilotDiagnosticCatalog.cs`, `PilotSymbol.cs` | Hover, completion, semantic tokens, go-to-def, code actions, quick fixes. |
| **VS Code Extension** | `vscode-pilot/src/extension.ts`, `vscode-pilot/syntaxes/pilot.tmLanguage.json`, `vscode-pilot/package.json` | LSP client, Run/Debug editor buttons, `launch.json` provider, TextMate grammar. |
| **CLI Driver & Interactive REPL** | `Cli/Program.cs`, `Cli/PilotRepl.cs` | All `pilot` subcommands and flags. |
| **Automated Tests** | `Pilot.Tests/*.cs`, `tests/*.pilot`, `Testing/PilotTestRunner.cs` | Run `dotnet test --filter FullyQualifiedName~<TestClass>` for fast targeted verification. |

### Cross-Layer Checklist When Adding/Modifying a Language Feature
1. **Grammar**: `antlr/Pilot.g4` (check `MILESTONES.md` Conflict Resolutions first; run `dotnet build Pilot.csproj` to regenerate ANTLR parser).
2. **Static Analysis**: `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (+ `PilotExhaustivenessChecker.cs` / `PilotBorrowChecker.cs` if applicable).
3. **AST Execution**: `Runtime/Execution/PilotInterpreterVisitor.cs` (+ `Runtime/Values/PilotValue.cs` if new value representation).
4. **Bytecode & VM**: `Runtime/Bytecode/PilotOpcode.cs` -> `Compiler/PilotBytecodeCompiler.cs` -> `Runtime/Bytecode/PilVirtualMachine.cs` (+ `PilotCompiledProgram.cs` if serialized).
5. **IDE / LSP**: `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` + `PilotDiagnosticCatalog.cs` (+ `vscode-pilot/syntaxes/pilot.tmLanguage.json` for highlighting).
6. **Codegen / Emitters (if applicable)**: `Native/*Emitter.cs` and `Transpilation/Targets/*TargetConverter.cs`.
7. **Tests**: Add xUnit tests in `Pilot.Tests/` and `.pilot` script in `tests/`.
<!-- END MANUAL NOTES -->
'@

$ManualNotes = $DefaultManualNotes
if (Test-Path $IndexPath) {
    $ExistingContent = [System.IO.File]::ReadAllText($IndexPath)
    if ($ExistingContent -match '(?s)(<!-- BEGIN MANUAL NOTES -->.*?<!-- END MANUAL NOTES -->)') {
        $ManualNotes = $Matches[1]
    }
}

$bt = '`'
$sb = [System.Text.StringBuilder]::new(65536)
[void]$sb.AppendLine('# Pilot — AI-Only Codebase Navigation & Symbol Index')
[void]$sb.AppendLine()
[void]$sb.AppendLine('> **STRICTLY FOR AI AGENTS**: Do not link this file in user-facing documentation. Use exact paths and `:L<line>` anchors below with `read_file` (`StartLine`/`EndLine`) to jump directly to code without exploratory directory/grep bouncing.')
[void]$sb.AppendLine("> **Auto-Sync Stamp**: ${bt}$StampHash${bt} | **Total Indexed Files**: $($AllFiles.Count) | **Regenerate**: ${bt}pwsh -NoProfile -File .agents/scripts/update_ai_index.ps1${bt}")
[void]$sb.AppendLine()
[void]$sb.AppendLine($ManualNotes.Trim())
[void]$sb.AppendLine()
[void]$sb.AppendLine('---')
[void]$sb.AppendLine()

# ==============================================================================
# SECTION 2: antlr/Pilot.g4 Grammar Rules & Lexer Tokens
# ==============================================================================
[void]$sb.AppendLine('## 2. Master Grammar Map (`antlr/Pilot.g4` Exact Line Anchors)')
[void]$sb.AppendLine()
$GrammarPath = Join-Path $RepoRoot "antlr\Pilot.g4"
if (Test-Path $GrammarPath) {
    $gLines = [System.IO.File]::ReadAllLines($GrammarPath)
    [void]$sb.AppendLine("Total lines in ${bt}antlr/Pilot.g4${bt}: **$($gLines.Length)**")
    [void]$sb.AppendLine()
    $parserRules = [System.Collections.Generic.List[string]]::new()
    $lexerTokens = [System.Collections.Generic.List[string]]::new()
    for ($i = 0; $i -lt $gLines.Length; $i++) {
        $ln = $i + 1
        $text = $gLines[$i]
        if ($text -cmatch '^([a-z][a-zA-Z0-9_]*)\s*(?::|$)') {
            $rName = $Matches[1]
            if ($rName -notin @('grammar', 'options', 'tokens', 'channels', 'import', 'fragment', 'mode')) {
                $hasColon = ($text -match ':')
                if (-not $hasColon -and ($i + 1) -lt $gLines.Length) {
                    for ($j = $i + 1; $j -le [Math]::Min($i + 3, $gLines.Length - 1); $j++) {
                        if ($gLines[$j] -match '^\s*:') { $hasColon = $true; break }
                        if ($gLines[$j].Trim().Length -gt 0 -and $gLines[$j] -notmatch '^\s*//') { break }
                    }
                }
                if ($hasColon) {
                    $parserRules.Add("${bt}${rName}${bt}:L${ln}")
                }
            }
        }
        elseif ($text -cmatch '^([A-Z][A-Z0-9_]*)\s*:') {
            $tName = $Matches[1]
            $lexerTokens.Add("${bt}${tName}${bt}:L${ln}")
        }
    }
    [void]$sb.AppendLine("### Parser Rules ($($parserRules.Count))")
    [void]$sb.AppendLine((Format-WrappedItems $parserRules))
    [void]$sb.AppendLine()
    [void]$sb.AppendLine("### Lexer Tokens ($($lexerTokens.Count))")
    [void]$sb.AppendLine((Format-WrappedItems $lexerTokens))
    [void]$sb.AppendLine()
}

[void]$sb.AppendLine('---')
[void]$sb.AppendLine()

# ==============================================================================
# SECTION 3: Mega-File Internal Method Maps (> 350 lines C# / TS files)
# ==============================================================================
[void]$sb.AppendLine('## 3. Large File Internal Method Maps (Files > 350 Lines)')
[void]$sb.AppendLine()
[void]$sb.AppendLine('Use these `:L<line>` method anchors to pass exact `StartLine`/`EndLine` into `read_file` for large files:')
[void]$sb.AppendLine()

$MethodRegex = [regex]::new('^\s*(?:public|private|protected|internal)\s+(?:static\s+|override\s+|virtual\s+|async\s+|sealed\s+|unsafe\s+|partial\s+|new\s+)*(?:[A-Za-z0-9_<>,\.\[\]\?\(\)\s]+?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[A-Za-z0-9_,\s]+>)?\s*\(', [System.Text.RegularExpressions.RegexOptions]::Compiled)
$SkipMethodNames = [System.Collections.Generic.HashSet[string]]::new([string[]]@('if', 'for', 'foreach', 'while', 'switch', 'catch', 'using', 'lock', 'return', 'nameof', 'typeof', 'sizeof'))

$CsAndTsFiles = $AllFiles | Where-Object { ($_.Ext -eq '.cs' -or $_.Ext -eq '.ts') -and $_.RelPath -notlike 'Pilot.Tests/*' }
foreach ($f in $CsAndTsFiles) {
    $lines = [System.IO.File]::ReadAllLines($f.FullPath)
    if ($lines.Length -le 350) { continue }

    $methods = [System.Collections.Generic.List[string]]::new()
    for ($i = 0; $i -lt $lines.Length; $i++) {
        $lineText = $lines[$i]
        if ($f.Ext -eq '.cs') {
            $m = $MethodRegex.Match($lineText)
            if ($m.Success) {
                $mName = $m.Groups[1].Value
                if (-not $SkipMethodNames.Contains($mName) -and $mName -notmatch '^(MyRegex|NPRegex|PyRegex|RustRegex|JsonRegex|FileIORegex|ConcurrencyRegex|PandasRegex)') {
                    $methods.Add("${bt}${mName}${bt}:L$($i + 1)")
                }
            }
        }
        elseif ($f.Ext -eq '.ts') {
            if ($lineText -match '^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z0-9_]+)\s*\(') {
                $methods.Add("${bt}$($Matches[1])${bt}:L$($i + 1)")
            }
        }
    }

    if ($methods.Count -gt 0) {
        [void]$sb.AppendLine("### ${bt}$($f.RelPath)${bt} ($($lines.Length) lines)")
        [void]$sb.AppendLine((Format-WrappedItems $methods))
        [void]$sb.AppendLine()
    }
}

[void]$sb.AppendLine('---')
[void]$sb.AppendLine()

# ==============================================================================
# SECTION 4: Complete Subsystem File & Type Inventory
# ==============================================================================
[void]$sb.AppendLine('## 4. Complete Codebase File & Type Symbol Inventory')
[void]$sb.AppendLine()

$TypeRegex = [regex]::new('^\s*(?:public|internal|private|protected|file)\s+(?:abstract\s+|sealed\s+|static\s+|partial\s+|readonly\s+|unsafe\s+|ref\s+)*(class|struct|interface|enum|record(?:\s+struct|\s+class)?)\s+([A-Za-z_][A-Za-z0-9_]*)', [System.Text.RegularExpressions.RegexOptions]::Compiled)

$Grouped = $AllFiles | Group-Object {
    $idx = $_.RelPath.IndexOf('/')
    if ($idx -gt 0) { $_.RelPath.Substring(0, $idx) } else { '(Repository Root)' }
} | Sort-Object Name

foreach ($grp in $Grouped) {
    [void]$sb.AppendLine("### $($grp.Name)")
    foreach ($f in $grp.Group) {
        $lines = [System.IO.File]::ReadAllLines($f.FullPath)
        $lineCount = $lines.Length
        $symbols = [System.Collections.Generic.List[string]]::new()

        if ($f.Ext -eq '.cs') {
            for ($i = 0; $i -lt $lineCount; $i++) {
                $tm = $TypeRegex.Match($lines[$i])
                if ($tm.Success) {
                    $kind = $tm.Groups[1].Value -replace '\s+', ' '
                    $tName = $tm.Groups[2].Value
                    $symbols.Add("$kind ${bt}${tName}${bt}:L$($i + 1)")
                }
            }
        }

        if ($symbols.Count -gt 0) {
            [void]$sb.AppendLine("- ${bt}$($f.RelPath)${bt} ($lineCount L) — " + ($symbols -join ', '))
        } else {
            [void]$sb.AppendLine("- ${bt}$($f.RelPath)${bt} ($lineCount L)")
        }
    }
    [void]$sb.AppendLine()
}

$OutDir = Split-Path -Parent $IndexPath
if (-not (Test-Path $OutDir)) {
    [void](New-Item -ItemType Directory -Path $OutDir -Force)
}

$NewContent = $sb.ToString().TrimEnd() + "`n"
$ShouldWrite = $true
if (Test-Path $IndexPath) {
    $OldContent = [System.IO.File]::ReadAllText($IndexPath)
    if ($OldContent -ceq $NewContent) {
        $ShouldWrite = $false
    }
}
if ($ShouldWrite) {
    [System.IO.File]::WriteAllText($IndexPath, $NewContent, [System.Text.UTF8Encoding]::new($false))
}
[System.IO.File]::WriteAllText($StampPath, $StampHash, [System.Text.UTF8Encoding]::new($false))
