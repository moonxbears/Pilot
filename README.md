# Pilot Programming Language

> **Type-Safe, Flexible, Configurable, Decoupled, Beautiful, Efficient, and Extendible.**

**Pilot** is a modern general-purpose programming language blending Pythonic elegance with C#-style robustness, native relational database operations, real-time determinism, polyglot interoperability, and universal package adoption.

---

## Table of Contents

- [Documentation & Reference Manuals](#documentation--reference-manuals)
- [Core Highlights](#core-highlights)
- [Design Philosophy](#design-philosophy)
- [Architecture & Repository Structure](#architecture--repository-structure)
- [Getting Started & Installation](#getting-started--installation)
- [Universal Package Manager (`pilot pkg`)](#universal-package-manager-pilot-pkg)
  - [Native Pilot Packages](#native-pilot-packages)
  - [Adopting Interop Packages (NuGet, npm, pip, Cargo, Maven, LuaRocks)](#adopting-interop-packages)
  - [Packaging CLI Reference](#packaging-cli-reference)
- [Virtual Environment System (`pilot venv`)](#virtual-environment-system-pilot-venv)
  - [Creating and Activating Environments](#creating-and-activating-environments)
  - [Virtual Environment CLI Reference](#virtual-environment-cli-reference)
  - [Automatic Dependency Sandboxing](#automatic-dependency-sandboxing)
  - [Programmatic Runtime API](#programmatic-runtime-api)
- [Language Features & Syntax Reference](#language-features--syntax-reference)
  - [1. Type Safety, Variables & Control Flow](#1-type-safety-variables--control-flow)
  - [2. Preconditions & Cleanup (`guard` and `defer`)](#2-preconditions--cleanup-guard-and-defer)
  - [3. Type Adapters, Argument Adaptation & Repeated Calls](#3-type-adapters-argument-adaptation--repeated-calls)
  - [4. Advanced Method Groups & Dispatch](#4-advanced-method-groups--dispatch)
  - [5. Algebraic Class Composition & Typestates](#5-algebraic-class-composition--typestates)
  - [6. Structural Projection & Scoped Lexical Extensions](#6-structural-projection--scoped-lexical-extensions)
  - [7. Contextual Parameters (Native Dependency Injection)](#7-contextual-parameters-native-dependency-injection)
  - [8. Design-by-Contract (`requires` and `ensures`)](#8-design-by-contract-requires-and-ensures)
  - [9. Exhaustive Pattern Matching Assignments](#9-exhaustive-pattern-matching-assignments)
  - [10. Relational & In-Memory Database Set Operations](#10-relational--in-memory-database-set-operations)
  - [11. Concurrency, Channels & Actors](#11-concurrency-channels--actors)
  - [12. Real-Time Determinism & Units of Measure](#12-real-time-determinism--units-of-measure)
  - [13. Metaprogramming, Macros & AST Generative Classes](#13-metaprogramming-macros--ast-generative-classes)
- [Polyglot Interoperability](#polyglot-interoperability)
- [Compilation & Scripting Runtime](#compilation--scripting-runtime)
- [Interactive Debugger & REPL](#interactive-debugger--repl)
- [Language Server Protocol (LSP) & Tooling](#language-server-protocol-lsp--tooling)
- [Running the Test Suites](#running-the-test-suites)
- [Step-by-Step Architecture Guide](#step-by-step-architecture-guide)

---

## Documentation & Reference Manuals

Explore the complete, navigable Pilot documentation suite in [`docs/index.md`](docs/index.md):

### Core Language Reference Manuals
- **[Master Milestones & Roadmap Tracker (`MILESTONES.md`)](MILESTONES.md)** — Dependency-verified roadmap and concurrency matrix tracking completed milestones (M1–M10), cross-milestone grammar/semantic conflict invariants, and upcoming strategic phases (M11–M27: exhaustive ADTs, concise `T?` null-safety, localized `T ! E` value errors, expression ergonomics & class-like literals, dynamic-schedule `for` loops & zero-marshalling C-ABI guarantees, bidirectional type inference, first-class keywords & homoiconic call-quoting, reactive/autodiff/dry-run projections, runtime-to-`comptime` demotion, accurate bidirectional `--emit`/`--from` transpilers & `pilot fmt` with `--viable-license-only`, bootstrapped & curated `stdlib`, DAP GUI breakpoints, deep LLVM/WASM linking, `pilot perf` auto-optimizer, self-hosted compiler, and package registry evaluation).
- **[Master Documentation Hub (`docs/index.md`)](docs/index.md)** — Central navigation portal, architecture pipeline, and cross-reference index.
- **[Exhaustive Lexical Reference (`docs/lexical-reference.md`)](docs/lexical-reference.md)** — Every lexer token, preprocessor directive (`#define`, `#if`, `#region`, `#pragma`), diagnostic code (`PREPROC001`–`PREPROC011`), keyword category, contextual identifier rule, numeric/dimensional literal (`50ms`, `100Hz`, `64KB`), string/f-string/symbol literal, and operator/delimiter.
- **[Exhaustive Syntax & Grammar Reference (`docs/syntax-reference.md`)](docs/syntax-reference.md)** — Every EBNF grammar production from `Pilot.g4`, program structure, `{}` vs `:` suites, type annotations, classes, algebraic class math, typestates, pattern matching, LINQ/SQL queries, concurrency, real-time blocks, macros, and the 19-level operator precedence table.
- **[Exhaustive Functions, Methods & Runtime API Reference (`docs/functions-and-apis.md`)](docs/functions-and-apis.md)** — Every global built-in function, built-in module (`json`, `venv`, `TypeAdapter`), scripting host API, Universal Root Object method, collection/matrix/generator/future/lazy method, Design-by-Contract intrinsic, standard contract (`IList`, `IEquatable`, etc.), and runtime exception type.

### 15 Deep-Dive Concept Guides (`docs/concepts/`)
1. **[Types, Variables, Collections & Control Flow](docs/concepts/01-types-variables-control-flow.md)**
2. **[Guard, Defer & Structured Exception Handling](docs/concepts/02-guard-defer-exceptions.md)**
3. **[First-Class Method Groups & Multi-Dispatch](docs/concepts/03-method-groups-and-dispatch.md)**
4. **[First-Class Type Adapters & Scalar-to-Collection Repetition](docs/concepts/04-type-adapters-and-repetition.md)**
5. **[Algebraic Class Composition & Typestate Classes](docs/concepts/05-algebraic-classes-and-typestates.md)**
6. **[Structural Projections, Scoped Lexical Extensions & Generative Classes](docs/concepts/06-projections-extensions-generative.md)**
7. **[Contextual Parameters & Lexical Dependency Injection](docs/concepts/07-contextual-parameters-di.md)**
8. **[Exhaustive Pattern Matching & Deep Destructuring](docs/concepts/08-exhaustive-pattern-matching.md)**
9. **[Design-by-Contract, Contract Delegation & Method Stealing](docs/concepts/09-design-by-contract.md)**
10. **[Native Database Engine, LINQ Queries & SQL-First DML](docs/concepts/10-database-and-sql.md)**
11. **[Concurrency, Stateful Generators & Lazy Evaluation](docs/concepts/11-concurrency-generators-lazy.md)**
12. **[Polyglot Interoperability & Resumable Yield-Bridge Coroutines](docs/concepts/12-polyglot-interoperability.md)**
13. **[Metaprogramming, Homoiconic Lisp & Universal Root Object](docs/concepts/13-metaprogramming-and-lisp.md)**
14. **[Hard Real-Time Systems, Dimensional Units & Native Matrix Types](docs/concepts/14-realtime-and-matrix-types.md)**
15. **[Universal Package Manager, Virtual Environments, Preprocessor & Tooling](docs/concepts/15-packaging-venv-and-tooling.md)**

---

## Core Highlights

- **Pythonic Syntax + C# Robustness**: Familiar syntax with properties (`get`/`set`), operator overloads, generic `where` constraints, interfaces, contract delegation, and method stealing.
- **Universal Package Manager**: First-class native packaging with full capability to adopt dependencies from **NuGet** (.NET), **npm** (Node), **pip** (Python), **Cargo** (Rust), **Maven** (Java), and **LuaRocks** (Lua).
- **Argument Type Adapters & Auto-Vectorization**: Adapt arguments to matching signatures (e.g. `str` -> `int`, `dict` -> `Model`), or repeat singular calls over collections via `each`, `..` spread, `*` broadcast, or `with auto_repeat`.
- **Method Groups**: Auto-routing switchboards over heterogeneous collections, currying signature sieves, fallback unions (`|`), and multi-cast broadcasts (`*`).
- **Algebraic Types & Typestates**: Compose classes using set algebra (`Admin = BaseUser | AdminPrivileges`, `FreeUser = BaseUser - BillingModule`), and morph object types based on state (`Connection[Disconnected]` -> `Connection[Connected]`).
- **Contextual Parameters**: Implicit downstream parameter propagation (`context db: Database`) bound via lexical `with context db = ...` blocks.
- **Design-by-Contract**: Declarative pre-conditions (`requires`) and post-conditions (`ensures`) with snapshot introspection (`old(expr)`).
- **Deep Polyglot Engine**: Direct inline block execution for JavaScript, Java, Lua, C, C++, Assembly, PostgreSQL, MySQL, Python, C#, and Rust.

---

## Design Philosophy

1. **Flexible**: Adaptable syntax, dynamic-to-static typing continuum, and seamless interop across 10+ programming ecosystems.
2. **Configurable**: Configurable execution contexts, extensible polyglot registries, and pluggable type adapters.
3. **Decoupled**: Distinct boundaries between lexical parsing (ANTLR4), runtime interpretation (`PilotInterpreterVisitor`), compilation (`PilotCompiler`), debugging (`PilotDebugger`), and language server services (`PilotLspServer`).
4. **Beautiful**: Clean, expressive syntax free of excessive boilerplate, inspired by Python, Rust, and modern C#.
5. **Efficient**: Low-overhead interpretation, compiled binary cache format (`.pilc`), and fast deterministic dispatch.
6. **Extendible**: Extensible via macros (`quote`/`quasiquote`), lexical extensions (`extension ... on ...`), and custom user type adapters (`adapter`).

---

## Architecture & Repository Structure

```
Pilot/
├── crates/
│   ├── pilot-arena/         # High-performance 64KB slab bump allocator & generational typed arenas
│   ├── pilot-span/          # Zero-copy SIMD SWAR source spans & fast LineIndex
│   ├── pilot-intern/        # Thread-safe lock-free symbol & keyword interner
│   ├── pilot-ast/           # Strictly 16-byte cache-line aligned AST nodes and flat AstArena
│   ├── pilot-lexer/         # Zero-allocation SIMD-accelerated lexer (140+ tokens)
│   ├── pilot-parser/        # Predictive recursive-descent & Pratt expression parser
│   ├── pilot-syntax/        # Rowan-style lossless concrete syntax tree (CST) & green trees
│   ├── pilot-fmt/           # Zero-allocation CST code formatter & style engine
│   ├── pilot-incremental/   # Salsa-style query memoization with early-cutoff recomputation
│   ├── pilot-db/            # In-memory relational database, table indexing, & query engine
│   ├── pilot-analysis/      # Hindley-Milner type inference, pattern exhaustiveness, & contracts
│   ├── pilot-perceus/       # Perceus reference counting with static reuse & drop specialization
│   ├── pilot-vm/            # 32-bit register bytecode PilVM v3 and JIT/interpreter
│   ├── pilot-codegen/       # LLVM IR, C11/C23, WASM, and multi-approach GPU emitters (CUDA/HIP/Vulkan/WGSL)
│   ├── pilot-transpile/     # High-fidelity bidirectional transpilation to 7 languages
│   ├── pilot-polyglot/      # Zero-copy FFI bridge for Python, JS, Lua, C, C++, Java, ASM, SQL
│   ├── pilot-dap/           # Debug Adapter Protocol (DAP) server & interactive terminal debugger
│   ├── pilot-lsp/           # JSON-RPC Language Server Protocol service & semantic analyzer
│   ├── pilot-pkg/           # Package manager & permissive SPDX license compliance verifier
│   ├── pilot-bench/         # Nanosecond benchmark harness & cross-language comparative runner
│   ├── pilot-perf/          # Profiler, worst-case execution time (WCET) analyzer, & CST auto-optimizer
│   ├── pilot-ffi/           # Stable C-ABI foreign function interface & C header exports
│   ├── pilot-rt/            # Zero-overhead runtime, networking, and memory primitives
│   └── pilot-cli/           # Unified toolchain CLI entrypoint (`pilot`)
├── examples/                # 32+ comprehensive showcase scripts demonstrating all language features
├── tests/                   # 36 end-to-end integration and differential semantic test suites
└── vscode-pilot/            # Production Visual Studio Code extension with native LSP & DAP
```

---

## Getting Started & Installation

### Prerequisites
- [Rust 1.80+](https://rustup.rs/) (Cargo toolchain)
- Optional: C/C++ compiler (`clang` or `gcc`) for Native AOT and LLVM IR compilation

### Build & Verification
```bash
# Clone the repository
git clone https://github.com/pilot-lang/pilot.git
cd Pilot

# Build release binaries (pilot and pilot-lsp)
cargo build --release

# Run all crate unit and integration test suites
cargo test --all

# Run all 36 end-to-end language test suites
cargo run --bin pilot -- test

# Run multi-backend differential semantic verification
cargo run --bin pilot -- test --differential
```

### CLI Command Summary
```bash
pilot run <file> [args...]                    # Execute a Pilot program directly via PilVM v3
pilot build <file> [options]                  # Compile to native AOT binary, LLVM IR, C, WASM, or GPU
  --target=native|llvm|c|wasm|gpu             # Select compilation backend (default: native)
  --burst                                     # Enable Burst SIMD vectorization pass
  --gpu --gpu-backend=cuda|rocm|vulkan|wgpu   # Multi-approach GPU kernel generation (NVIDIA & AMD)
  -o, --output <file>                         # Set output binary path
  -O0 | -O2 | -O3                             # Optimization level
pilot check <file>                            # Zero-allocation syntax and semantic analysis
pilot fmt <file>                              # Format source files according to style invariants
pilot test [filter] [--diff]                  # Discover and run unit, integration, or differential suites
pilot repl                                    # Start interactive Pilot Read-Eval-Print Loop
pilot pkg <subcommand>                        # Universal Package Manager & SPDX license verifier
pilot venv <subcommand>                       # Manage isolated Pilot virtual environments
pilot debug [--dap]                           # Launch debug adapter or interactive debugger
pilot lsp                                     # Native Language Server Protocol (LSP) service
pilot perf <subcommand>                       # Performance profiling, WCET, & CST auto-optimizer
pilot bench                                   # Hardware throughput & cross-language comparative benchmark
pilot --emit <lang> <file>                    # Transpile Pilot to C, C++, C#, Rust, Python, JS, or Lua
pilot --from <lang> <file>                    # Ingest foreign source code into canonical Pilot syntax
```

---

## Universal Package Manager (`pilot pkg`)

Pilot includes a complete, production-grade package manager capable of managing native Pilot packages and **adopting packages from foreign ecosystems**.

### Native Pilot Packages
Initialize a new project:
```bash
pilot pkg init my-awesome-app
```
This generates a `pilot.json` manifest:
```json
{
  "name": "my-awesome-app",
  "version": "0.1.0",
  "description": "Package my-awesome-app created with Pilot package manager.",
  "main": "main.pilot",
  "dependencies": {},
  "interopDependencies": {
    "nuget": {},
    "npm": {},
    "pip": {},
    "cargo": {},
    "maven": {},
    "luarocks": {}
  }
}
```

Add a native Pilot dependency:
```bash
pilot pkg add pilot-http@1.2.0
```

### Adopting Interop Packages
Pilot allows you to adopt packages from foreign package managers directly into your Pilot project:

```bash
# Adopt a .NET NuGet package
pilot pkg add nuget:Newtonsoft.Json@13.0.3

# Adopt a JavaScript npm package
pilot pkg add npm:lodash@4.17.21

# Adopt a Python PyPI package
pilot pkg add pip:requests@2.31.0

# Adopt a Rust Cargo crate
pilot pkg add cargo:serde@1.0

# Adopt a Java Maven artifact
pilot pkg add maven:org.json:json@20231013

# Adopt a Lua LuaRocks module
pilot pkg add luarocks:luasocket@3.0
```

### Packaging CLI Reference

| Command | Description |
| :--- | :--- |
| `pilot pkg init [name]` | Creates a new `pilot.json` manifest in the current directory |
| `pilot pkg add <pkgSpec>` | Adds a native or interop dependency (`[ecosystem:]package[@version]`) |
| `pilot pkg install` / `restore` | Restores all declared dependencies and writes `pilot.lock.json` |
| `pilot pkg remove <pkgSpec>` | Removes a dependency from `pilot.json` and lockfile |
| `pilot pkg list` | Lists all installed native and adopted interop dependencies |
| `pilot pkg pack [out.pilpkg]` | Bundles the project into a distributable `.pilpkg` archive |

---

## Virtual Environment System (`pilot venv`)

Pilot includes a complete, production-ready virtual environment system that isolates runtime environments, installed native modules, and foreign dependencies across projects.

### Environment Layout

When created, a Pilot virtual environment initializes a deterministic, self-contained filesystem layout:

```
myenv/
├── pilotvenv.cfg              # Key-value environment configuration
├── Scripts/                   # (bin/ on Unix/Linux/macOS)
│   ├── activate.ps1           # PowerShell activation script
│   ├── activate.bat           # Windows Command Prompt activation script
│   ├── deactivate.bat         # Windows Command Prompt deactivation script
│   ├── activate               # POSIX Bash/Zsh activation script
│   └── pilot.cmd / pilot      # Isolated binary launcher
├── lib/
│   └── .pilot_modules/        # Sandboxed module directory
│       ├── pilot/             # Native Pilot packages
│       ├── nuget/             # Adopted NuGet packages
│       ├── npm/               # Adopted npm packages
│       ├── pip/               # Adopted Python wheels
│       ├── cargo/             # Adopted Cargo crates
│       ├── maven/             # Adopted Java JARs
│       └── luarocks/          # Adopted Lua modules
└── include/                   # C/C++ header stubs for native extensions
```

### Creating and Activating Environments

**Create a virtual environment:**
```bash
# Basic creation in .venv
pilot venv create .venv

# Custom prompt name and include system-wide packages
pilot venv create /path/to/myenv --prompt dev-sandbox --system-site-packages
```

**Activate your environment:**
```powershell
# PowerShell
& .venv\Scripts\activate.ps1

# Windows CMD
.venv\Scripts\activate.bat

# Linux / macOS (POSIX)
source .venv/bin/activate
```

Once activated, your terminal prompt is prefixed with `(myenv)` and any `pilot pkg install` or `pilot pkg add` automatically targets the isolated `.venv/lib/.pilot_modules/` location.

**Deactivate:**
```bash
deactivate
```

### Virtual Environment CLI Reference

| Command | Description |
| :--- | :--- |
| `pilot venv create <path> [--prompt <name>] [--system-site-packages]` | Creates a new virtual environment at the specified directory |
| `pilot venv info [path]` | Inspects the currently active or detected virtual environment configuration |

### Automatic Dependency Sandboxing

When a virtual environment is active (detected via `PILOT_VENV` or `VIRTUAL_ENV`), the Pilot Universal Package Manager (`pilot pkg`) automatically switches its module resolution root to:
```
<venv_path>/lib/.pilot_modules
```
This guarantees that projects running different versions of native packages, NuGet assemblies, npm packages, or Python libraries do not conflict or contaminate system directories.

### Programmatic Runtime API

Pilot provides first-class language bindings to manage and query virtual environments directly within Pilot code:

```pilot
import venv # or virtual_environment

# Query activation status
let active = venv.is_active();
print("Is active: " + active);

# Query current environment root path
let env_path = venv.active_path();
print("Active venv path: " + env_path);

# Create a virtual environment programmatically
venv.create("./.custom_venv", "my-script-env");

# Programmatically activate or deactivate in-process
venv.activate("./.custom_venv");
print("Now running inside: " + venv.active_path());

venv.deactivate();
```

---

## Language Features & Syntax Reference

### 1. Type Safety, Variables & Control Flow

```pilot
let greeting : str = "Hello, Pilot!";
var counter : int = 0;
const MAX_LIMIT = 1000;

for i in 0..5:
    counter += i;

if counter > 5:
    println("Counter exceeded 5: " + str(counter));
```

### 2. Preconditions & Cleanup (`guard` and `defer`)

`guard` enforces early-exit preconditions, while `defer` guarantees LIFO cleanup execution:

```pilot
def process_file(path: str):
    guard path.length > 0 else:
        return "invalid_path";

    println("Opening file: " + path);
    defer println("Closing file resource.");
    defer println("Flushing pending buffers.");

    return "processed";
```

### 3. Type Adapters, Argument Adaptation & Repeated Calls

Pilot allows passing a different type into a signature when possible, adapting arguments or unrolling collections into repeated individual calls:

```pilot
let myarr = [];
let myitems : list = [1, 2, 3, 4, 5];

# Option 1: 'each' argument modifier
myarr.append(each myitems);

# Option 2: Collection spread '..' inline
myarr.append(..myitems);

# Option 3: Broadcast operator '*'
myarr.append*(myitems);

# Option 4: Scoped 'with auto_repeat' block
with auto_repeat:
    myarr.append(myitems);

# Repeated calls on functions returning values collects results into a list:
def square(x: int): return x * x;
let squares = square(each [1, 2, 3]); # -> [1, 4, 9]

# Language-level Type Adapters
adapter str -> int:
    return int(self);

def calc(x: int): return x * 10;
let val = calc("42"); # Automatically adapts "42" -> 42 -> 420
```

### 4. Advanced Method Groups & Dispatch

Method groups provide unified dispatch over function overloads:

```pilot
# Overloaded method group
def process(data: int): return data * 2;
def process(data: str): return data.to_upper();
def process(data: list): return data.length;

# 1. Switchboard Auto-Routing over heterogeneous collections:
let mixed = [42, "hello", [1, 2, 3], 99];
let routed = mixed.map(process); # [84, "HELLO", 3, 198]

# 2. Currying (The Signature Sieve):
def draw(shape: Circle, color: Color): ...
def draw(shape: Square, color: Color): ...
def draw(shape: Triangle, angle: float): ...

const draw_red = draw(color=Color.RED); # Narrowed group of (Circle) and (Square)

# 3. Group Fallback & Union (|):
const serialize_any = Json.serialize | Xml.serialize;
serialize_any(my_object);

# 4. Multi-Cast Execution (*):
on_event*(PlayerJoined("Isaac")); # Invokes all matching event overloads
```

### 5. Algebraic Class Composition & Typestates

```pilot
class BaseUser:
    let id = 1;

class AdminPrivileges:
    def ban(self, uid) { return "banned"; }

class BillingModule:
    def pay(self, amount) { return "paid"; }

# Set Algebra on classes:
class Admin = BaseUser | AdminPrivileges;      # Union
class FreeUser = BaseUser - BillingModule;      # Subtraction
class Common = AdminPrivileges & BillingModule; # Intersection

# Typestate Classes (Morphing Types):
class Connection:
    state Disconnected:
        def connect(self) -> Self[Connected]: ...
    state Connected:
        def send(self, data): ...
        def close(self) -> Self[Disconnected]: ...

let conn = Connection();
let active = conn.connect();
active.send("payload");
```

### 6. Structural Projection & Scoped Lexical Extensions

```pilot
# Structural Projection (->)
class Employee:
    let name = "Isaac";
    let department = "Engineering";
    let salary = 120000;

let dto = employee -> { name, department };
# dto is: { "name": "Isaac", "department": "Engineering" }

# Scoped Lexical Extensions (Monkey Patching with Scope Safety)
extension StringHelpers on str:
    def double(self): return self + self;

with extension StringHelpers:
    println("hi".double()); # "hihi"
# Outside the block, 'double' is not present on str
```

### 7. Contextual Parameters (Native Dependency Injection)

Contextual parameters are implicitly propagated downstream without boilerplate passing:

```pilot
def save_user(user, context db: Database):
    db.execute("INSERT", user);

def register_user(payload, context db: Database):
    save_user(payload["username"]); # db implicitly forwarded

with context db = Database("postgresql://localhost:5432"):
    register_user({ "username": "Isaac" });
```

### 8. Design-by-Contract (`requires` and `ensures`)

```pilot
def transfer(amount: float, source: Account, dest: Account)
    requires amount > 0
    requires source.balance >= amount
    ensures source.balance == old(source.balance) - amount:

    source.deduct(amount);
    dest.add(amount);
```

### 9. Exhaustive Pattern Matching Assignments

```pilot
# Record pattern matching destructuring
{ "id": userId, "metadata": { "role": role } } = fetch_user_record();

# List pattern matching with slice rest spread
[head, ..tail] = [10, 20, 30, 40];
```

### 10. Relational & In-Memory Database Set Operations

```pilot
database InMemoryStore:
    table users (id int primary key, name str, active bool);

# LINQ-style query with grouping and ordering
let activeAdmins = from u in users
                   where u.active == true
                   order by u.name ascending
                   select u.name;

# Postgres SQL operations
transaction:
    insert into users values (1, "Isaac", true)
    on conflict do nothing;
    savepoint sp1;
    commit;
```

### 11. Concurrency, Channels & Actors

```pilot
actor Worker:
    def handle(self, task):
        return "completed:" + task;

let ch = channel(10);
spawn:
    ch <- "message";

select:
    case msg = <-ch:
        println("Received: " + msg);
    timeout 500ms:
        println("Timed out waiting for channel.");
```

### 12. Real-Time Determinism & Units of Measure

```pilot
# Dimensional units of measure
let dist = 100 [m];
let duration = 9.58 [s];
let speed = dist / duration; # speed has dimensional unit [m/s]

# Real-time execution budgets
@hard_realtime
@wcet(50us)
def telemetry_tick():
    noalloc:
        # Guarantee zero GC allocation within bounded cycle
        pass;
```

### 13. Metaprogramming, Macros & AST Generative Classes

```pilot
# Compile-time AST macro
macro log_expr(expr):
    quote:
        println("Result of expression is: " + str(expr));

# Generative Class Blocks
class DatabaseModel:
    generate:
        for field in ["id", "username", "created_at"]:
            inject_field(field);
```

---

## Polyglot Interoperability

Pilot provides native inline blocks for 10+ language ecosystems:

```pilot
# 1. SQL with Structured Row Objects
let users = postgresql {
    select id, name, count(*) from accounts where id in (1, 2, 3);
}
for u in users:
    println(u.name);

# 2. Python Interop with Variable Sharing & Yielding
let myVariable = "Hello from Pilot!";
python {
    print(myVariable)
}

# 3. Inline JavaScript, C, C++, Lua, and Assembly
let jsVal = js {
    let x = 10;
    return x * 4;
}

let luaVal = lua {
    return 100
}

let asmVal = asm {
    mov eax, 42
}
```

---

## Compilation & Scripting Runtime

Pilot functions as both a zero-overhead scripting engine and an ahead-of-time precompiler:

```bash
# 1. Scripting mode (Direct execution)
pilot my_script.pilot

# 2. Ahead-Of-Time Precompilation (.pilot -> .pilc)
pilot compile my_script.pilot -o my_script.pilc

# 3. Direct execution of precompiled binary
pilot my_script.pilc
```

---

## Interactive Debugger & REPL

Pilot features a complete interactive command-line debugger with source tracking and breakpoint control:

```bash
pilot debug my_script.pilot
```

Debugger Commands:
- `b <line>`: Set breakpoint at line
- `c` / `continue`: Continue execution
- `n` / `step`: Step over to next statement
- `s` / `stepin`: Step into function
- `o` / `stepout`: Step out of function
- `l` / `locals`: Inspect local variables in current stack frame
- `bt` / `stack`: Print full call stack trace
- `q` / `quit`: Terminate debugger

Interactive REPL:
```bash
pilot repl
```

---

## Language Server Protocol (LSP) & Tooling

Pilot includes a standalone, zero-GC native JSON-RPC 2.0 Language Server communicating over standard I/O:

```bash
pilot lsp
# or directly via binary:
pilot-lsp
```

Features:
- **23-Type Semantic Tokens Provider**: High-fidelity semantic highlighting with modifier bitmasks (`declaration`, `definition`, `readonly`, `static`, `defaultLibrary`).
- **Interactive Hover & Signature Help**: Type hints, docstrings, and active parameter indexing for built-in, polyglot, and user-defined functions.
- **Incremental CST & Early Cutoff Synchronization**: Sub-millisecond Salsa-style query recomputation (`pilot-incremental` and `pilot-db`).
- **VS Code Extension**: Located in `vscode-pilot/` with full Language Server Protocol (LSP) and Debug Adapter Protocol (DAP) integration.

---

## Running the Test Suites

Pilot is validated through unit test suites across all 24 crates, 36 end-to-end language integration test suites, and multi-backend differential verification:

```bash
# Run all crate unit & integration tests
cargo test --all

# Run all 36 language integration test suites
pilot test

# Run multi-backend differential semantic verification (PilVM vs Native AOT vs LLVM vs WASM)
pilot test --differential
```

---

## Step-by-Step Architecture Guide

```mermaid
flowchart TD
    Source[".pilot Source File / Code String"] --> Lexer["pilot-lexer\n(Zero-Alloc SIMD Lexer)"]
    Lexer --> Parser["pilot-parser\n(Pratt & Recursive Descent)"]
    Parser --> CST["pilot-syntax\n(Rowan Lossless CST)"]
    Parser --> AST["pilot-ast\n(16-Byte Flat AstArena)"]
    
    AST --> Analysis["pilot-analysis\n(HM Type Inference, Contracts & Exhaustiveness)"]
    AST --> Perceus["pilot-perceus\n(Zero-GC Reference Counting & Reuse)"]
    
    Analysis --> ModeCheck{"Execution Backend"}
    
    ModeCheck -->|PilVM v3 Bytecode| VM["pilot-vm\n(32-Bit Register Bytecode VM)"]
    ModeCheck -->|Native AOT| CCodeGen["pilot-codegen (C11/C23 Native Driver)"]
    ModeCheck -->|LLVM IR| LLVM["pilot-codegen (LLVM IR Direct Emitter)"]
    ModeCheck -->|WebAssembly| WASM["pilot-codegen (WASM + WASI)"]
    ModeCheck -->|GPU Kernels| GPU["pilot-codegen (CUDA, ROCm, Vulkan, WGSL)"]
    ModeCheck -->|Transpilation| Transpile["pilot-transpile (C, C++, C#, Rust, Python, JS, Lua)"]
    
    Source --> Incremental["pilot-incremental & pilot-db\n(Salsa Early-Cutoff Engine)"]
    Incremental --> LSP["pilot-lsp\n(23-Type Semantic Tokens & Hover)"]
    AST --> DAP["pilot-dap\n(Debug Adapter Protocol Server)"]
```

1. **Zero-Allocation Lexing (`pilot-lexer`)**:
   Transforms UTF-8 source characters into SIMD-accelerated token streams with string-interning and zero heap allocations.
2. **Predictive Parsing & Lossless CST (`pilot-parser` & `pilot-syntax`)**:
   Constructs a Rowan-style lossless Concrete Syntax Tree (`GreenNode`) and simultaneously lowers into cache-packed 16-byte `AstNode` entries in `AstArena`.
3. **Static Analysis & Type Inference (`pilot-analysis`)**:
   Performs bidirectional Hindley-Milner type inference, pattern exhaustiveness checking, Design-by-Contract verification, and static `@comptime` demotion.
4. **Perceus Linear Reference Counting (`pilot-perceus`)**:
   Inserts compile-time drop specializations and in-place buffer reuses, guaranteeing zero-GC pauses and real-time determinism.
5. **Multi-Backend Code Generation (`pilot-codegen` & `pilot-vm`)**:
   Executes directly in the 32-bit register `PilVirtualMachine`, or compiles to Native AOT binaries (Clang/GCC), LLVM IR, WebAssembly (WASI), or GPU compute shaders (CUDA/HIP/Vulkan/WGSL).
6. **Polyglot Execution (`pilot-polyglot`)**:
   Bridges foreign execution blocks (`python`, `js`, `lua`, `c`, `cpp`, `java`, `asm`, `sql`) with zero-marshalling shared-memory `SliceBuffer<T>`.
7. **IDE Services (`pilot-lsp` & `pilot-dap`)**:
   Provides incremental LSP diagnostics, semantic highlighting, hover cards, code formatting, and DAP interactive debugging.
6. **Package Resolution (`PilotPackageManager`)**:
   Loads native Pilot modules and adopted interop dependencies from `.pilot_modules/` or system feeds based on `pilot.json` and `pilot.lock.json`.
