# Concept 15: Universal Package Manager, Virtual Environments, Preprocessor & Tooling

[← Prev: Real-Time & Matrix Types](./14-realtime-and-matrix-types.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md)

---

## 1. Concept Overview & Design Rationale

Pilot includes a complete, batteries-included developer toolchain built directly into the `pilot` CLI and runtime:

1. **Universal Polyglot Package Manager (`pilot pkg`)**: Manages both native `.pilpkg` archives and foreign ecosystem dependencies (`nuget:`, `npm:`, `pip:`, `cargo:`, `maven:`, `luarocks:`) through a single manifest (`pilot.json`) and deterministic SHA-256 lockfile (`pilot.lock.json`).
2. **First-Class Virtual Environments (`pilot venv` & `venv` runtime API)**: Creates isolated `.pilot_venv` sandboxes (`pilot venv create`) with cross-platform activation scripts (`activate.ps1`, `activate.bat`, `activate`) and exposes programmatic environment management (`venv.create`, `venv.activate`, `venv.info`) directly inside Pilot scripts.
3. **Full C#/C-Style Preprocessor**: Supports `#define`, `#undef`, `#if`/`#elif`/`#else`/`#endif` (with `&&`, `||`, `!`, `==`, `!=`, `defined()`), `#region`/`#endregion`, `#warning`, `#error`, `#pragma`, `#nullable`, and `#line` while preserving exact 1:1 source line numbers.
4. **AOT Compilation (`pilot compile`), Scripting API, CLI Debugger & LSP**: Compile scripts to standalone `.pilc` bundles (`pilot compile`), debug interactively (`pilot debug`), explore in the REPL (`pilot --repl`), or use the Language Server (`Pilot.LanguageServer`) in VS Code.

---

## 2. Exact Lexical Tokens & Preprocessor Directives Used

| Directive / Command | Syntax | Purpose |
| :--- | :--- | :--- |
| `#define` / `#undef` | `#define DEBUG`, `#define LEVEL 3`, `#undef DEBUG` | Defines or removes preprocessor symbols in `PilotPreprocessor` |
| `#if` / `#elif` / `#else` / `#endif` | `#if defined(DEBUG) && LEVEL >= 2` | Conditional compilation branches (blanking inactive lines to preserve line numbers) |
| `#region` / `#endregion` | `#region Helpers ... #endregion` | Collapsible code folding regions (validated for balanced nesting) |
| `#warning` / `#error` | `#warning msg`, `#error msg` | Emits `PREPROC008` warning or halts execution with `PREPROC007` error |
| `#pragma` / `#nullable` / `#line` | `#pragma warning disable PREPROC008` | Controls diagnostic suppression, nullability state, and virtual line mapping |
| `pilot pkg` | `init`, `add`, `install`, `remove`, `list`, `pack` | Universal polyglot package manager CLI subcommands |
| `pilot venv` | `create [path]`, `info [path]` | Virtual environment creation and inspection CLI subcommands |

---

## 3. Runtime & Toolchain Mechanics

### 3.1 Universal Package Manager (`Packaging/PilotPackageManager.cs`)
- **`pilot.json` Manifest**: Tracks package metadata (`name`, `version`, `entryPoint`) and ecosystem-prefixed dependencies:
  - `"pilot:std-collections"` (Native Pilot package)
  - `"nuget:Newtonsoft.Json"` (.NET NuGet adapter)
  - `"npm:lodash"` (Node.js npm adapter)
  - `"pip:numpy"` (Python PyPI adapter)
  - `"cargo:serde"` (Rust Crates.io adapter)
  - `"maven:org.apache.commons:commons-lang3"` (Java Maven adapter)
  - `"luarocks:luasocket"` (LuaRocks adapter)
- **Virtual Environment Sandboxing**: When a `.pilot_venv` is active (either via `PILOT_VIRTUAL_ENV` or auto-discovered in `<workspace>/.pilot_venv`), `pilot pkg add` and `pilot pkg install` automatically sandbox all installed modules inside `<venv>/.pilot_modules/` instead of the root directory!

### 3.2 First-Class Virtual Environments (`Packaging/PilotVirtualEnvManager.cs`)
A Pilot virtual environment directory (`.pilot_venv` by default) contains:
```text
.pilot_venv/
├── pilot-venv.json        # Environment name, creation timestamp, runtime version, isolation settings
├── .pilot_modules/        # Isolated package installation store
└── bin/
    ├── activate.ps1       # PowerShell activation script
    ├── activate.bat       # Windows CMD activation script
    └── activate           # POSIX sh/bash/zsh activation script
```
Moreover, Pilot scripts can create, inspect, activate, and deactivate virtual environments at runtime via the built-in `venv` (or `virtual_environment`) module!

### 3.3 AOT Compilation, Debugger, REPL & Scripting Host
- **AOT Compiler (`Compilation/PilotCompiler.cs`)**: `pilot compile app.pilot -o app.pilc` validates the syntax tree and packages the preprocessed source, metadata, and SHA-256 integrity hash into a portable `.pilc` binary bundle runnable via `pilot run-compiled app.pilc` (or `pilot app.pilc`).
- **Interactive CLI Debugger (`Debugging/PilotCliDebugger.cs`)**: `pilot debug app.pilot` attaches an execution hook to `PilotExecutionContext.StepHook`, supporting breakpoints (`break <line>`, `delete <line>`), line stepping (`step`, `next`, `continue`), stack inspection (`bt`), scope inspection (`locals`), and live expression evaluation (`print <expr>`, `eval <expr>`).
- **Scripting Host (`Scripting/PilotScript.cs`)**: Executes `.pilot` files as automation scripts with injected globals (`args`, `argv`, `script_path`, `env`, `cwd`) and file/process helpers (`read_text`, `write_text`, `file_exists`, `delete_file`, `exit`).

---

## 4. Available Functions, Modules & CLI Commands

### Built-In `venv` / `virtual_environment` Runtime Module
| Function / Property | Signature | Description |
| :--- | :--- | :--- |
| `venv.is_active` | `bool` (property) | `true` if a virtual environment is currently active. |
| `venv.active_path` | `str \| none` (property) | Absolute path to the active virtual environment, or `none`. |
| `venv.modules_path` | `str` (property) | Path to the active `.pilot_modules` directory. |
| `venv.create` | `venv.create([path, name, include_global]) -> map` | Creates a new virtual environment and returns its configuration map. |
| `venv.activate` | `venv.activate([path]) -> map` | Activates the virtual environment at `path` for the current process. |
| `venv.deactivate` | `venv.deactivate() -> bool` | Deactivates the current in-process virtual environment. |
| `venv.info` | `venv.info([path]) -> map` | Returns status (`exists`, `is_active`, `path`, `modules_path`, `installed_packages`, `package_count`). |
| `venv.list_packages` | `venv.list_packages([path]) -> list[str]` | Returns a list of package identifiers installed in the environment. |

### `pilot` CLI Reference Summary
| Command | Description |
| :--- | :--- |
| `pilot <file.pilot> [-- args...]` | Preprocesses and executes a `.pilot` script. |
| `pilot --repl` | Starts the interactive multi-line Pilot REPL (`:help`, `:vars`, `:reset`, `:load`, `:type`, `:ast`, `:tokens`, `:quit`). |
| `pilot debug <file.pilot>` | Starts the interactive step-by-step CLI debugger. |
| `pilot compile <file.pilot> [-o out.pilc]` | Compiles a `.pilot` file into a `.pilc` bundle. |
| `pilot run-compiled <file.pilc>` | Executes a compiled `.pilc` bundle. |
| `pilot --emit <python\|csharp\|rust> <file.pilot>` | Transpiles Pilot code to Python, C#, or Rust. |
| `pilot --from <python\|csharp\|rust\|js> <file>` | Converts foreign source code into Pilot syntax. |
| `pilot pkg <init\|add\|install\|remove\|list\|pack>` | Manages native and polyglot dependencies. |
| `pilot venv <create\|info> [path]` | Creates or inspects a Pilot virtual environment. |

---

## 5. Complete Runnable Examples

See [`examples/showcase/15_virtual_environments_and_packaging.pilot`](../../examples/showcase/15_virtual_environments_and_packaging.pilot) and [`tests/14_preprocessing_directives.pilot`](../../tests/14_preprocessing_directives.pilot) for the full executable showcases.

```pilot
#define FEATURE_TELEMETRY
#define API_VERSION 2

// 1. Conditional compilation via the Pilot Preprocessor
let mode = "unknown"
#if defined(FEATURE_TELEMETRY) && API_VERSION >= 2
mode = "telemetry_v2"
#else
mode = "legacy"
#endif

assert(mode == "telemetry_v2")

// 2. Programmatic Virtual Environment inspection via the built-in 'venv' module
let status = venv.info()
assert(status.contains_key("is_active"))
assert(status.contains_key("modules_path"))
assert(type_of(venv.list_packages()) == "list")

// 3. Built-in JSON serialization module
let manifest_preview = json.dumps({
    "name": "demo-service",
    "mode": mode,
    "packages": venv.list_packages()
})
let roundtrip = json.loads(manifest_preview)
assert(roundtrip["name"] == "demo-service" and roundtrip["mode"] == "telemetry_v2")
```

---

[← Prev: Real-Time & Matrix Types](./14-realtime-and-matrix-types.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md)
