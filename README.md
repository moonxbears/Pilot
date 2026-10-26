# Pilot Language Specification & ANTLR4 Grammar

**Pilot** is a type-safe programming language blending Pythonic syntax with C#-style robustness, featuring:
- **Native & Postgres-Style Database Integration**: In-memory LINQ queries, indexed table lookups, and generic SQL constructs (`INSERT ON CONFLICT`, `RETURNING`, transactions, savepoints, and window functions).
- **Asynchronous & Multi-Threaded Concurrency**: Actors, Go-style channels (`select`), thread spawning, `parallel for`, `atomic`, and `sync` blocks.
- **Real-Time Trade-offs**: Hard real-time (`@hard_realtime`, `@wcet`, `noalloc`, `stackalloc`, `bounded` loops) and soft real-time (`deadline`, `within ... fallback`, `periodic` tasks).
- **Universal Domain-Specific Abstractions (UDSA)**: First-class dimensional units of measure, compile-time AST macros (`quote`), and functional pipeline operators (`|>`).
- **C#-like Robustness**: Properties with `get`/`set`, operator overloading, extension methods, and generic `where` constraints.
- **Preconditions & Cleanup**: Mandatory early-exit `guard` and function-exit `defer` statements.

---

## Repository Structure & Grammar Architecture

```
Pilot/
├── antlr/
│   ├── Pilot.g4                 <-- MASTER COMPILER GRAMMAR (Used by C# & Rider Preview)
│   ├── core/                    <-- Domain: Pythonic Core, Type Safety, Guard & Defer
│   │   ├── CoreLexer.g4
│   │   └── CoreParser.g4        (Single Source of Truth for base language rules)
│   ├── database/                <-- Domain: In-Memory & Postgres Database Operations
│   │   ├── DatabaseLexer.g4
│   │   └── DatabaseParser.g4    (imports CoreParser)
│   ├── concurrency/             <-- Domain: Async, Channels, Actors, Threading
│   │   ├── ConcurrencyLexer.g4
│   │   └── ConcurrencyParser.g4 (imports CoreParser)
│   ├── realtime/                <-- Domain: Hard/Soft Real-Time & Units of Time/Freq/Mem
│   │   ├── RealtimeLexer.g4
│   │   └── RealtimeParser.g4    (imports CoreParser)
│   ├── udsa/                    <-- Domain: Universal Domain-Specific Abstractions & Macros
│   │   ├── UDSALexer.g4
│   │   └── UDSAParser.g4        (imports CoreParser)
│   └── csharp/                  <-- Domain: Properties, Operator Overloads, Extensions
│       ├── CSharpLexer.g4
│       └── CSharpParser.g4      (imports CoreParser)
├── examples/                    <-- Sample Pilot scripts demonstrating language features
│   └── overview.pilot
├── tests/                       <-- Integration test suite for all language capabilities
│   ├── 01_type_safety.pilot
│   ├── 02_database_set_operations.pilot
│   ├── 03_concurrency.pilot
│   ├── 04_realtime.pilot
│   ├── 05_udsa_metaprogramming.pilot
│   ├── 06_syntax_edge_cases.pilot
│   ├── 07_syntax_errors.pilot
│   ├── 08_guard_defer.pilot
│   ├── 09_postgres_database.pilot
│   └── 10_csharp_robustness.pilot
├── Pilot.Tests/                 <-- xUnit automated test suite (39 unit tests)
│   ├── Pilot.Tests.csproj
│   └── PilotLanguageTests.cs
├── Class1.cs                    <-- Pilot CLI runner & parse tree visualizer
├── Pilot.csproj                 <-- Main C# project compiling Pilot.g4 via Antlr4 MSBuild
└── README.md
```

---

## Active Grammars vs. Obsolete Files (What to Use)

### 1. Active Master Grammar: `antlr/Pilot.g4`
- **What it is**: The complete, production combined grammar.
- **Where it is used**:
  - Automatically compiled by MSBuild during `dotnet build` into `PilotLexer` and `PilotParser`.
  - Used in Rider / IntelliJ ANTLR plugin to display full hierarchical multi-level parse trees for `.pilot` files.

### 2. Active Modular Domain Grammars: `antlr/<domain>/`
- **What they are**: Domain-separated specifications for clear organizational structure.
- **Rule of DRY (Don't Repeat Yourself)**:
  - `antlr/core/CoreParser.g4` is the **Single Source of Truth** for core language constructs (`identifier`, `test`, `suite`, `parameters`, `type_spec`, etc.).
  - All domain parsers (`database`, `concurrency`, `realtime`, `udsa`, `csharp`) use `import CoreParser;`. There are zero duplicate stubs or redefinitions.

### 3. Removed Remnants (Cleaned Up)
The following old artifacts from earlier iterations have been removed:
- ❌ `BaseGrammar.g4`, `BaseLexar.g4`, `BaseLexar.tokens` (obsolete early prototypes)
- ❌ `antlr/grammar/` and `antlr/lexar/` folders (obsolete split folders)
- ❌ `test_folders/` (temporary experiment directory)
- ❌ `BaseGrammar` / `BaseLexar` compatibility wrappers in C# code (now strictly using `PilotParser` and `PilotLexer`)

---

## Building and Testing

### Run xUnit Automated Tests
```powershell
dotnet test
```
*Executes all 39 comprehensive unit tests covering type safety, database queries, concurrency, real-time deadlines, UDSA macros, guard/defer, and C# robustness.*

### Run All Script Integration Tests
```powershell
dotnet run -- --all
```
*Parses all 10 scripts in `tests/` and validates syntax verification and error detection.*

### Inspect a Parse Tree in the CLI
```powershell
dotnet run -- --tree examples/overview.pilot
```

### Inspect Lexer Tokens in the CLI
```powershell
dotnet run -- --tokens examples/overview.pilot
```
