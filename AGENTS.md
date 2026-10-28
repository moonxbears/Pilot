# Pilot Workspace — AI Navigation & Engineering Rules

> **AI-ONLY CONFIGURATION**: `AGENTS.md` and `.agents/AI_CODEBASE_INDEX.md` exist strictly for AI coding assistants so you can jump directly to exact files and line numbers without exploratory bouncing. Never add links to these files in user-facing docs (`README.md`, `docs/`).

## Mandatory Navigation & Accuracy Protocol

1. **Zero Blind Bouncing — Read `.agents/AI_CODEBASE_INDEX.md` First**:
   - Before searching directories or scanning multi-thousand-line files, read `.agents/AI_CODEBASE_INDEX.md` (`StartLine`/`EndLine` chunks of ~400 lines):
     - **Section 1**: Task-to-File Routing Table & Cross-Layer Feature Checklist.
     - **Section 2**: Exact `:L<line>` anchors for every parser rule & lexer token in `antlr/Pilot.g4`.
     - **Section 3**: Exact `:L<line>` method maps inside every large file (`> 350` lines), including `PilotSemanticAnalyzer.cs` (~4.3k L), `PilotInterpreterVisitor.cs` (~4.2k L), `PilotExhaustivenessChecker.cs` (~1.8k L), `PilotBytecodeCompiler.cs` (~1.4k L), `PilotValue.cs` (~1.2k L), `Cli/Program.cs` (~1.2k L), `PilotLlvmEmitter.cs`, `PilVirtualMachine.cs`, and `PilotWasmEmitter.cs`.
     - **Section 4**: Complete file & `class`/`struct`/`enum`/`interface`/`record` symbol inventory across all subsystems and tests.
2. **Keep the AI Index Constantly Updated**:
   - `.agents/hooks.json` (`SessionStart` + `PostToolUse`) automatically invokes `.agents/scripts/update_ai_index.ps1 -CheckStampOnly` whenever source files change, keeping all line numbers, grammar anchors, methods, and file lists synchronized.
   - If you introduce a **new top-level subsystem directory** or **new compiler pass**, also update the curated table inside `<!-- BEGIN MANUAL NOTES --> ... <!-- END MANUAL NOTES -->` in `.agents/AI_CODEBASE_INDEX.md` (and `AGENTS.md` below), and run `pwsh -NoProfile -File .agents/scripts/update_ai_index.ps1` if running in an environment without lifecycle hooks.
3. **Check `MILESTONES.md` & Architectural Invariants**:
   - Consult `MILESTONES.md` for active milestone status (`M1`–`M27`), parallel execution waves, and the **7 Cross-Milestone Conflict Resolutions** (e.g., no unchecked postfix `x!`; postfix `?` is exclusively early-return propagation; `..`/`..=` inclusive vs. `..<` half-open; `#` strictly reserved and never used in call-quoting tokens).

---

## Instant Subsystem Jump Map

| Task / Domain | Go Directly To |
| :--- | :--- |
| **Grammar & Lexer** | `antlr/Pilot.g4`, `Compiler/PilotParserFast.cs` |
| **Preprocessor (`#define`, `#if`, `#pragma`)** | `Preprocessing/PilotPreprocessor.cs` |
| **AST Interpreter & Pattern Runtime** | `Runtime/Execution/PilotInterpreterVisitor.cs`, `PilotEngine.cs`, `PilotPatternMatcher.cs` |
| **Runtime Values, ADTs, Objects, Keywords** | `Runtime/Values/PilotValue.cs`, `PilotValueKind.cs`, `PilotAdt.cs`, `PilotObject.cs`, `PilotMethodGroup.cs`, `PilotKeyword.cs`, `PilotFunction.cs`, `PilotMatrix.cs` |
| **Bytecode Compiler & PilVM** | `Runtime/Bytecode/PilotOpcode.cs`, `Compiler/PilotBytecodeCompiler.cs`, `Compiler/PilotCompiledProgram.cs`, `Runtime/Bytecode/PilVirtualMachine.cs`, `PilotBytecodeChunk.cs` |
| **Type Inference, Exhaustiveness, Borrow, Loops, Units, Contracts, Call-Quoting, Autodiff, Signals, Dry-Runs** | `Analysis/PilotAnalysisPipeline.cs`, `Analysis/TypeSystem/PilotTypeInferenceEngine.cs`, `Analysis/TypeSystem/PilotBidirectionalInferencer.cs`, `PilotType.cs`, `Analysis/CallQuoting/PilotCallQuotingEngine.cs`, `Analysis/Autodiff/PilotSymbolicDifferentiator.cs`, `Runtime/Reactive/PilotSignalGraph.cs`, `Runtime/Effects/PilotEffectDryRunEngine.cs`, `Analysis/PatternMatching/PilotExhaustivenessChecker.cs`, `Analysis/Ownership/PilotBorrowChecker.cs`, `Analysis/Loops/PilotForLoopAnalyzer.cs`, `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs`, `Analysis/ContractVerification/PilotContractVerifier.cs` |
| **Built-in Functions & Scripting Host** | `Runtime/Builtins/PilotBuiltinRegistry.cs`, `Scripting/PilotScriptContext.cs`, `Scripting/PilotScript.cs` |
| **Adapters, Extensions & Runtime Contracts** | `Runtime/Adapters/PilotTypeAdapterRegistry.cs`, `Runtime/Extensions/PilotLexicalExtension.cs`, `Runtime/Contracts/PilotContractRegistry.cs` |
| **Compile-Time `@` Source Generators** | `CodeGen/Engine/PilotCodeGeneratorPipeline.cs`, `CodeGen/Builtins/*.cs` |
| **Bidirectional Transpilers (`--emit` / `--from`)** | `Transpilation/TranspilationRegistry.cs`, `PilotLibraryTranspiler.cs`, `PilotLibraryMapper.cs`, `PilotOptimizer.cs`, `Transpilation/Targets/*TargetConverter.cs` |
| **Native AOT (LLVM, WASM, C-ABI, ASM) & Zero-Copy Shared Memory** | `Native/PilotNativeCompiler.cs`, `Native/Llvm/PilotLlvmEmitter.cs`, `Native/Wasm/PilotWasmEmitter.cs`, `Native/CAbi/PilotCAbiEmitter.cs`, `Native/Assembly/PilotAssemblyEmitter.cs`, `Native/Runtime/pilot_runtime.h`, `Runtime/Burst/PilotBurstEngine.cs`, `Runtime/NativeMemory/PilotSharedBuffer.cs`, `Runtime/NativeMemory/PilotLoopAbiTypes.cs` |
| **Polyglot Interop (`python {}`, `rust {}`, SQL)** | `Runtime/Interop/PolyglotRegistry.cs`, `PolyglotTypeBridge.cs` |
| **Packages (`pilot pkg`) & Virtual Envs (`pilot venv`)** | `Runtime/Packaging/PilotPackageManager.cs`, `Runtime/Packaging/Adapters/BaseInteropPackageAdapter.cs`, `Runtime/Venv/PilotVirtualEnvironment.cs` |
| **Debugger & Hot Reload** | `Debugging/PilotDebugger.cs`, `Debugging/PilotCliDebugger.cs`, `Runtime/HotReload/PilotHotReloader.cs` |
| **LSP Server & Semantic Diagnostics** | `Pilot.LanguageServer/Lsp/PilotLspServer.cs`, `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs`, `PilotDiagnosticCatalog.cs`, `PilotSymbol.cs` |
| **VS Code Extension** | `vscode-pilot/src/extension.ts`, `vscode-pilot/syntaxes/pilot.tmLanguage.json`, `vscode-pilot/package.json` |
| **CLI Subcommands & REPL** | `Cli/Program.cs`, `Cli/PilotRepl.cs` |
| **Tests (Fast Targeted Runs)** | `Pilot.Tests/*.cs` (`dotnet test --filter FullyQualifiedName~<Class>`), `tests/*.pilot` |

## Cross-Layer Checklist for Language Changes
1. `antlr/Pilot.g4` $\rightarrow$ `dotnet build Pilot.csproj` (regenerates ANTLR C# lexer/parser in `obj/`).
2. `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (+ `PilotExhaustivenessChecker.cs` / `PilotBorrowChecker.cs`).
3. `Runtime/Execution/PilotInterpreterVisitor.cs` (+ `Runtime/Values/PilotValue.cs`).
4. `Runtime/Bytecode/PilotOpcode.cs` $\rightarrow$ `Compiler/PilotBytecodeCompiler.cs` $\rightarrow$ `Runtime/Bytecode/PilVirtualMachine.cs`.
5. `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` + `PilotDiagnosticCatalog.cs` + `vscode-pilot/syntaxes/pilot.tmLanguage.json`.
6. `Native/*Emitter.cs` & `Transpilation/Targets/*TargetConverter.cs` (when target lowering applies).
7. `Pilot.Tests/*.cs` + `tests/*.pilot` + update `MILESTONES.md` checkboxes.
