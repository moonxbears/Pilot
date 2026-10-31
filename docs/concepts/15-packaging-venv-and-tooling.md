# Concept 15: Universal Package Manager, Virtual Environments, Preprocessor & Tooling

[← Prev: Real-Time & Matrix Types](./14-realtime-and-matrix-types.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md)

---

## 1. Concept Overview & Design Rationale

Pilot includes a complete, batteries-included developer toolchain built directly into the `pilot` CLI and runtime:

1. **Universal Polyglot Package Manager (`pilot pkg`)**: Manages both native `.pilpkg` archives and foreign ecosystem dependencies (`nuget:`, `npm:`, `pip:`, `cargo:`, `maven:`, `luarocks:`) through a single manifest (`pilot.json`) and deterministic SHA-256 lockfile (`pilot.lock.json`).
2. **First-Class Virtual Environments (`pilot venv` & `venv` runtime API)**: Creates isolated `.pilot_venv` sandboxes (`pilot venv create`) with cross-platform activation scripts (`activate.ps1`, `activate.bat`, `activate`) and exposes programmatic environment management (`venv.create`, `venv.activate`, `venv.info`) directly inside Pilot scripts.
3. **Full C#/C-Style Preprocessor**: Supports `#define`, `#undef`, `#if`/`#elif`/`#else`/`#endif` (with `&&`, `||`, `!`, `==`, `!=`, `defined()`), `#region`/`#endregion`, `#warning`, `#error`, `#pragma`, `#nullable`, and `#line` while preserving exact 1:1 source line numbers.
4. **Ahead-Of-Time Precompilation (`pilot compile`), Stdlib Cache & Native C-ABI Tooling**: Compile scripts to optimized zero-deserialization `.pilc` v3 bytecode binaries (`pilot compile`), precompile all 40 standard library modules (`pilot compile --stdlib`), link against native C-ABI runtime libraries (`libpilot_rt.a` / `libpilot_rt.so`), transpile across 6 verified target languages (`pilot --emit`), debug interactively (`pilot debug`), explore in the REPL (`pilot repl`), or run the native Language Server (`pilot lsp`).

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

### 3.1 Universal Package Manager (`crates/pilot-pkg`)
- **`pilot.json` Manifest**: Tracks package metadata (`name`, `version`, `entryPoint`) and ecosystem-prefixed dependencies:
  - `"pilot:std-collections"` (Native Pilot package)
  - `"nuget:Newtonsoft.Json"` (.NET NuGet adapter)
  - `"npm:lodash"` (Node.js npm adapter)
  - `"pip:numpy"` (Python PyPI adapter)
  - `"cargo:serde"` (Rust Crates.io adapter)
  - `"maven:org.apache.commons:commons-lang3"` (Java Maven adapter)
  - `"luarocks:luasocket"` (LuaRocks adapter)
- **Virtual Environment Sandboxing**: When a `.pilot_venv` is active (either via `PILOT_VIRTUAL_ENV` or auto-discovered in `<workspace>/.pilot_venv`), `pilot pkg add` and `pilot pkg install` automatically sandbox all installed modules inside `<venv>/.pilot_modules/` instead of the root directory!

### 3.2 First-Class Virtual Environments (`crates/pilot-pkg` & `crates/pilot-rt`)
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

### 3.3 AOT Precompilation, Stdlib Cache & Native C-ABI Linking

#### 32-Bit Register PilVM Bytecode Precompiler (`crates/pilot-vm`)
- **`pilot compile app.pilot -o app.pilc`**: Validates the syntax tree, lowers to the 32-bit register bytecode instruction set, and writes an optimized, portable `.pilc` v3 binary.
- **Zero-Deserialization `.pilc` v3 Specification**:
  1. **Magic Header (`6 bytes`)**: `PILC\x03\x00` ensuring format version verification.
  2. **Constant Pool**: Tagged 64-bit integer, float, boolean, and small-string-optimized (`TAG_SSO_STR`) literals.
  3. **Heap String Table**: Length-prefixed UTF-8 byte payloads for strings $> 7$ bytes. During deserialization, safe runtime heap strings are rehydrated into in-process address space, eliminating cross-process dangling pointer faults.
  4. **Instruction Section**: Aligned 12-byte 32-bit register bytecode instructions with opcode, destination register, and operand fields.
- **Sub-Millisecond Cold Starts**: Precompiled `.pilc` binaries execute via `pilot <file.pilc>` or `pilot run <file.pilc>` with $< 0.2\text{ ms}$ startup time.

#### Standard Library Precompilation (`pilot compile --stdlib`)
Precompiles all **40 native standard library modules** into `stdlib/.pilc-cache/`:
- Generates canonical modules: `stdlib/.pilc-cache/std.<mod>.pilc`
- Generates content-addressable cache entries: `stdlib/.pilc-cache/std.<mod>_<sha256>.pilc`
- `PilotStdlibResolver` automatically looks up precompiled `.pilc` artifacts first, falling back to `.pilot` source parsing only if the cache is missing or modified.

#### Linkable Native C-ABI Runtime Libraries (`crates/pilot-rt`)
Compiled Pilot code and external C/C++ applications link against native artifacts generated by `pilot-rt`:
- **`libpilot_rt.a`**: High-performance static library compiled with Fat LTO and hardware SIMD acceleration.
- **`libpilot_rt.so`**: Lightweight dynamic shared library (383 KB).
- **`libpilot_ffi.so`**: Stable FFI foreign function interface bridge.
- **`pilot_runtime.h`**: Clean C99/C11 header declaring Tier 1 bump regions (`pilot_rt_region_*`), Tier 2 Perceus linear reference counting (`pilot_rt_alloc_arc`, `pilot_rt_dup`, `pilot_rt_drop`, `pilot_rt_drop_reuse`), high-precision timers, and cryptographic hashing.

#### Interactive CLI Debugger & Language Server
- **Interactive CLI Debugger (`crates/pilot-dap`)**: `pilot debug app.pilot` attaches execution hooks supporting breakpoints (`b <line>`), line stepping (`n`), stack backtraces (`bt`), local variables (`l`), and live evaluation.
- **Native JSON-RPC Language Server (`crates/pilot-lsp`)**: `pilot lsp` provides incremental Salsa-style diagnostics, 23-type semantic tokens, hover cards, and code formatting over standard I/O.

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
| `pilot run <file> [args...]` | Executes a `.pilot` source script or `.pilc` binary via PilVM v3. |
| `pilot <file> [args...]` | Shorthand execution of `.pilot` source or `.pilc` precompiled binary. |
| `pilot compile <file.pilot> [-o out.pilc]` | Precompiles a `.pilot` source file into optimized `.pilc` v3 bytecode. |
| `pilot compile --stdlib` | Precompiles all 40 standard library modules into `stdlib/.pilc-cache/`. |
| `pilot build <file> [options]` | Compiles Pilot source to native AOT binary, LLVM IR, C, WASM, or GPU. |
| `pilot check <file>` | Zero-allocation syntax and semantic analysis. |
| `pilot fmt <file>` | Formats source files according to style invariants. |
| `pilot test [filter] [--diff]` | Discovers and executes unit, integration, and differential semantic test suites. |
| `pilot repl` | Starts the interactive multi-line Pilot REPL. |
| `pilot debug <file.pilot>` | Starts the interactive step-by-step CLI debugger. |
| `pilot lsp` | Starts the native JSON-RPC Language Server Protocol service. |
| `pilot perf <subcommand>` | Performance profiling, WCET analysis, and CST auto-optimizer. |
| `pilot bench` | Nanosecond throughput benchmarks and cross-language comparative analysis. |
| `pilot --emit <lang> <file> [-o out]` | Transpiles Pilot to C, C++, C#, Rust, Python, JavaScript, or Lua. |
| `pilot --from <lang> <file> [-o out]` | Ingests foreign source code into canonical Pilot syntax. |
| `pilot pkg <subcommand>` | Universal polyglot package manager CLI (`init`, `add`, `install`, `remove`, `list`, `pack`). |
| `pilot venv <subcommand>` | Virtual environment management CLI (`create`, `info`). |

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
