# Pilot Programming Language

> **Self-Hosted, Zero-GC, Type-Safe, Configurable, Decoupled, and Multi-Target.**

**Pilot** is a modern systems and application programming language built on an **Engine-Positive 14-Token Rust Core** that self-bootstraps its type system, operators, control flow, and standard library in pure Pilot (`stdlib/core/`). Pilot unifies C#-grade ergonomics (interfaces, implicit properties, operator hierarchies, `[...]` attribute pipelines, LINQ/SQL) with deterministic 3-tier Perceus memory management, zero-marshalling `SliceBuffer<T>` / `stride<T>` C-ABI layouts, homoiconic call-quoting (`!`, `!!`, `!&`, `!^`, `!@`, `!?`), polyglot interoperability, and native compilation to **PilVM v3 Bytecode, C11/C23 (Clang), LLVM IR, WebAssembly (WASI), x86_64/ARM64 ASM, and NVIDIA/AMD/WebGPU compute kernels**.

---

## Core Architectural Highlights

- **14-Token Seed Engine + 4-Stage Self-Genesis (`stdlib/core/`):** The Rust lexer hardcodes strictly **14 seed tokens** (`__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`). At Line 1 of `stdlib/core/core.pilot`, a compile-time `Cfg` table registers all modifiers, Pratt operator precedences, `[...]` attributes, and SSA intrinsics, then permanently seals the bootstrap bridge (`seal_bootstrap_bridge`).
- **Single Origin (`IAnyValue`) & 2-Bit Hardware Type Lattice:** Every type originates from `IAnyValue` with **0 runtime header bytes** and is classified in 1 CPU cycle via bitwise-OR of its fields and `~self()` destructor:
  - **`0b00` `IValue`:** Bit-copyable inline data (`i64`, `f64`, `Vec3`, `array<T, N>`, `RangeWindow<T>`).
  - **`0b01` `IReference`:** Borrowed zero-copy memory views (`slice<T>`, `stride<T>`, `str`, `map<K, V>`, `ref<T>`).
  - **`0b10` `IObject`:** Owned handles with deterministic `~self()` teardown (`SliceBuffer<T>`, `List<T>`, `Signal<T>`, `iso<T>`).
  - **`0b11` `IGuard`:** Scoped RAII tokens combining borrowed views with `~self()` cleanup (`MutexGuard<T>`, `ArenaScope`).
- **4-Tier `MethodTable`, `*Bundle` & `*Facet` Composition:** Replaces vtable-heavy OOP base classes with `interface` contracts, auto-implemented default methods, field-bearing `*Bundle` splices, and method-only `*Facet` splices with explicit compile-time collision resolution (`coalesce`, `with`, `without`).
- **7-Pillar `[...]` Attribute Engine:** Supports weighted execution pipelines (`IAttribute`), SSA entry/exit interceptors (`IInterceptor`), computed property formulas (`IFormulaAttribute`), reactive setter hooks (`IReactiveAttribute`), hierarchical inheritance (`IInheritedAttribute`), and ECS rule scheduling (`IEcsRule`).
- **Homoiconic Call-Quoting & Semantic Projections:** Zero-whitespace call-quoting operators project any function call into an AST descriptor (`fn!(a, b)`), constant-folded deep splice (`fn!!(~a, b)`), zero-marshalling `#[repr(C)]` C-ABI call frame (`fn!&(a, b)`), reactive signal (`fn!^(a, b)`), compile-time symbolic derivative (`fn!@(a, b)`), or Copy-on-Write dry-run plan (`fn!?(a, b)`).
- **Worry-Free 3-Tier Deterministic Zero-GC Memory:** Combines 16-byte `PilotVal` registers (with 14-byte inline SSO strings and niche-unboxed `T?` / `T ! E`), 64KB thread-local lexical bump arenas, and Perceus linear ARC with `drop_reuse` in-place recycling (`0.0 ms` GC pauses).
- **Deep Polyglot Interop, 7-Target Transpilation & Universal Packaging:** Execute inline `python`, `js`, `lua`, `c`, `cpp`, `rust`, `java`, `asm`, and `postgresql`/`sqlite` blocks, transpile bidirectionally (`pilot --emit` / `pilot --from`), and manage native or foreign packages (`NuGet`, `npm`, `pip`, `Cargo`, `Maven`, `LuaRocks`) via `pilot pkg` and `pilot venv`.

---

## Canonical Syntax & Type Reference

### 1. Single-Word `lowercase` Primitives & Symbolic Shorthands

| Symbolic Form | Canonical `lowercase` Type | Lattice | Backing Interface / C-ABI Layout |
| :--- | :--- | :--- | :--- |
| `T[]` | `slice<T>` | `0b01` | `IReadOnlyList<T>` (`{ ptr<T>, usize }`, 16B) |
| `T[N]` | `array<T, const N: usize>` | `0b00` | `IList<T>` (Inline `N * sizeof(T)` bytes) |
| `T[..]` | `stride<T>` | `0b01` | `IReadOnlyList<T>` (`{ ptr<T>, usize, isize }`, 24B) |
| `T*` | `ptr<T>` | `0b01` | `IIndexGet<usize, T>`, `IIndexSet<usize, T>` (8B) |
| `&T` / `&var T` | `ref<T>` / `var ref<T>` | `0b01` | Lexical immutable / mutable borrow |
| `~T` | `iso<T>` | `0b10` | Unique Perceus heap/arena ownership handle |
| `T?` | `option<T>` | `0b00*` | `ITryOperator`, `ICoalesceOperator` (Niche-unboxed) |
| `T ! E` | `result<T, E>` | `0b00*` | `ITryOperator` (Must-use register return) |
| `(A, B) => R` | `fn<(A, B), R>` | `0b01` | Callable closure view (`{ ptr<void>, ptr<void> }`) |
| `a ..< b` | `range<T>` | `0b00` | `IRangeOperator<T>` (Half-open `[a, b)`) |
| `a .. b` / `a ..= b` | `span<T>` | `0b00` | `ISpanOperator<T>` (Inclusive `[a, b]`) |
| `a .. b : s` | `step<T>` | `0b00` | `IStepOperator<T>` (Stepped sequence) |
| `!&call(a, b)` | `frame<TSig>` | `0b00` | Zero-marshalling C-ABI call frame (`#[repr(C)]`) |

### 2. Distributive Headers, Implicit Properties & 7-Pillar `[...]` Attributes

```pilot
[EnforceCoreNaming]
pub pkg Pilot.Core.Showcase;

pub fn distributive_demo() {
    let List<i32> {
        myarr1 = [1, 2, 3],
        myarr2 = [4, 5, 6,],
    };

    var Vec3<f64> {
        a = (1.0, 2.0, 3.0),
        b = (9.0, 8.0, 7.0,),
    };
}

pub struct CombatUnit : IValue {
    pub:
        base_atk:  f32 = 50.0;
        crit_mult: f32 = 1.5;

        // Self-backed implicit property (`field` & `value`) with reactive attributes:
        [Clamp(min: 0.0, max: 1000.0, weight: 100), OnChange((old, cur) => Log.info(f"HP: {old} -> {cur}"))]
        hp: f32 = 100.0 {
            get => field;
            set => { field = value; }
        }

        // Formula Evaluator synthesizes computed getter at compile time:
        [Formula((u) => u.base_atk * u.crit_mult)]
        effective_atk: f32 { get; }

        [TraceSpan(name: "apply_damage", weight: 50)]
        var fn apply_damage(dmg: f32)
            requires dmg >= 0.0
            ensures .hp <= old(.hp)
        {
            .hp = .hp - dmg;
        }
}
```

# 3. Values-First Errors (T ! E), switch & 3-Tier Dynamic for Loops
## Code snippet

```pilot
pub fn process_window(inputs: str[]) -> i64[] ! ParseError {
    // 3-tier expression-oriented `for` loop yielding into a zero-marshalling `SliceBuffer<i64>`:
    let scaled: i64[] = for (i in 0 ..< inputs.count) {
        let val: i64 = inputs[i].trim().parse_i64()?;
        if val < 0 {
            continue 0 .. 2, ...; // Pushes remaining window onto `RangeWindow<usize>` worklist!
        }
        yield switch val {
            case 0       => 1;
            case 1 .. 10 => val * 10;
            default      => val;
        };
    };
    return scaled;
}
```

# Repository Structure
``` Plaintext
Pilot/
├── .agents/                 # AI codebase index (`AI_CODEBASE_INDEX.md`), hooks & sync scripts
├── crates/
│   ├── pilot-arena/         # 64KB slab bump allocator & typed generational arenas
│   ├── pilot-span/          # Zero-copy SIMD SWAR byte spans & LineIndex
│   ├── pilot-intern/        # Lock-free symbol & `:ns/kw` keyword interner
│   ├── pilot-lexer/         # 14-token seed SIMD lexer, `#if` preprocessor & `bracket_links`
│   ├── pilot-syntax/        # Rowan-style lossless Red/Green CST & panic-free recovery
│   ├── pilot-ast/           # 16-byte cache-packed `AstNode` & flat `AstArena`
│   ├── pilot-parser/        # Interleaved recursive-descent + Pratt parser & `ParserTables`
│   ├── pilot-analysis/      # 2-bit lattice, 4-tier `MethodTable`, HM/row inference, `comptime` `Cfg`,
│   │                        # autodiff (`!@`), call-quoting (`!&`), loops, dimensions & contracts
│   ├── pilot-perceus/       # 2-bit lattice-driven Perceus ARC, escape analysis & realtime verifier
│   ├── pilot-vm/            # 32-bit register `PilVM` v3 & zero-deserialization `.pilc` v3 cache
│   ├── pilot-codegen/       # LLVM IR, C11/C23 (Clang), x86_64/ARM64 ASM, WASM & GPU (CUDA/ROCm/Vulkan/WGSL)
│   ├── pilot-transpile/     # Bidirectional transpiler (`--emit` / `--from`) across 7 languages
│   ├── pilot-polyglot/      # Zero-copy `SliceBuffer` FFI & SQL bridges (Python, JS, Lua, C/C++, Rust, Postgres)
│   ├── pilot-db/            # In-memory relational database, B-Tree indexing & LINQ/SQL engine
│   ├── pilot-fmt/           # Lossless Red/Green CST formatter & syntax migration engine
│   ├── pilot-incremental/   # Salsa-style early-cutoff query memoization
│   ├── pilot-perf/          # Multi-target profiler, WCET analyzer & CST auto-optimizer
│   ├── pilot-pkg/           # Universal package manager, foreign interop & `pilot venv`
│   ├── pilot-lsp/           # Native JSON-RPC 2.0 Language Server (+ `pilot-lsp-macros`)
│   ├── pilot-dap/           # Native Debug Adapter Protocol server & interactive CLI debugger
│   ├── pilot-ffi/           # C-ABI compiler bridge
│   ├── pilot-rt/            # Linkable zero-GC C-ABI runtime (`libpilot_rt.a` / `pilot_runtime.h`)
│   ├── pilot-bench/         # Cross-language nanosecond benchmark suite
│   └── pilot-cli/           # Unified `pilot` toolchain binary
├── stdlib/
│   ├── core/                # 4-Stage Self-Genesis (`core.pilot`, `operator.pilot`, `builtins.pilot`, `mem.pilot`, `meta.pilot`)
│   └── ...                  # 40 standard library modules (`collections`, `math`, `io`, `net`, `http`, `crypto`, `json`, etc.)
├── selfhost/                # Self-hosted Pilot compiler (`lexer`, `parser`, `analyzer`, `emitter`, `pilotc.pilot`)
├── tools/pilot-kernel/      # Jupyter Notebook kernel (`pilot_kernel`)
├── tests/                   # 38 end-to-end integration suites & 6 multi-backend differential suites
├── examples/showcase/       # 34 interactive concept showcases
└── vscode-pilot/            # Visual Studio Code extension with native LSP & DAP integration
```

# Build, Verification & CLI Summary

```Bash
# Build release binaries (`pilot`, `pilot-lsp`, `pilot-dap`)
cargo build --release

# Run all workspace unit tests & phase verification gates
cargo test --workspace

# Precompile `stdlib/` into content-addressed `.pilc` v3 bytecode cache
pilot compile --stdlib

# Run all 38 integration suites & multi-backend differential verification
pilot test
pilot test --differential

# Native AOT, LLVM IR, C11/C23, WASM & Multi-Vendor GPU compilation
pilot build app.pilot --target=native|llvm|c|wasm|gpu [-O3] [--burst]
pilot build kernel.pilot --gpu --gpu-backend=cuda|rocm|vulkan|wgpu

# Lossless CST formatting, profiling, packaging & virtual environments
pilot fmt <path>
pilot perf profile|analyze|optimize <file.pilot>
pilot pkg init|add|install|pack
pilot venv create .venv

# Bidirectional transpilation (`c`, `cpp`, `csharp`, `rust`, `python`, `js`, `lua`)
pilot --emit rust app.pilot -o app.rs
pilot --from python --strict-pilot --viable-license-only lib.py -o lib.pilot
```