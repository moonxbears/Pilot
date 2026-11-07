# Pilot Programming Language

> **Self-Hosted (Stage-3 Fixed-Point), Name-Blind 14-Token Seed, Deterministic Zero-GC, Type-Safe, and Multi-Target.**

**Pilot** (`v1.1.51`) is a modern systems and application programming language built on an irreducible **14-Token Name-Blind Rust Hardware & Physics Engine** (`5,312` LOC across 12 crates) that self-bootstraps over **90% of its compiler, syntax grammar, type library, memory verifier, code emitters, LSP/DAP tooling, and standard library in pure Pilot** (`stdlib/` and `selfhost/`).

Pilot unifies C#-grade ergonomics (`interface` hierarchies, implicit `field`/`value` properties & indexers, operator contracts, `[...]` attribute pipelines, distributive headers, Fluid Cascades `..`, native LINQ/SQL) with deterministic **2-Bit Hardware Lattice Perceus ARC + lexical arenas (`0.0 ms` GC pauses)**, 6 homoiconic call-quoting projections (`!`, `!!`, `!&`, `!^`, `!@`, `!?`), bidirectional 7-language source transpilation, and native compilation to **PilVM v3 (`.pilc`), C11/C23 (`clang`), LLVM IR, WebAssembly (`WASI`), x86_64/ARM64 assembly, and 8 GPU compute targets**.

---

## Architectural Highlights

- **The Strict 14-Token Boundary & Universal Name-Blind Contract:**
  The Rust seed engine hardcodes strictly **14 tokens** (`__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`) and **zero** primitive or composite type names. Every scalar, slice, option, result, range, closure, and call-quote frame is derived strictly from 5 name-blind mechanisms:
  1. **Register & Layout Physics (`[Bits]` & `[Layout]`):** Deduces CPU register classes (`gpr_signed`, `gpr_unsigned`, `simd_float`, `simd_vector`, `raw_ptr`, `unit`, `noreturn`), bit widths, inline array `repeat`, and `niche` packing directly from layout descriptors.
  2. **Structural Field Anatomy (`TypeDef.fields` & `LatticeBits`):** Deduces contiguous views (`{ ptr<T>, usize }` + `0b01`), strided lenses (`{ ptr<T>, usize, isize }` + `0b01`), owned buffers (`{ ptr<T>, usize, usize }` + `0b10`), and bounds pairs (`{ T, T }` + `0b00`) from byte offsets and lattice bits.
  3. **Pure Pilot Homoiconic Syntax Rewrites (`syntax` & `syntax type_construct`):** Rewrites symbolic types (`T[]`, `T[N]`, `T[..]`, `T@N`, `T?`, `T!E`, `(A) => R`) and surface statements (`for`, `while`, `until`, `switch`, `guard`, `try`/`catch`, `select`, `requires`/`ensures`) in `stdlib/core/syntax/*.pilot`.
  4. **Operator & Protocol Contracts (`MethodTable`):** Deduces postfix `expr?` (`ITryOperator`), iteration (`get_enumerator()`), and `implicit` value lifting from resolved method tables.
  5. **`TypeId` Receptor Slots (`bind_receptor` & `nameof`):** Binds lexed literals and homoiconic call-quote projections to registered `TypeId` handles by structural field index (`0`, `1`, ...).
  - *Verified continuously by the **Universal Rename Litmus Gate**: renaming `char` $\rightarrow$ `c32`, `slice` $\rightarrow$ `window`, `option` $\rightarrow$ `maybe`, `result` $\rightarrow$ `outcome`, and `range` $\rightarrow$ `interval` in `stdlib/core/` compiles and emits valid code across all backends with zero Rust source changes.*

- **Single Origin (`IAnyValue`) & 2-Bit Hardware Type Lattice:**
  Every type originates from `IAnyValue` (**0 runtime header bytes**) and is classified in 1 CPU cycle via bitwise-OR of its stored fields and `~self()` destructor (`Bit 0` = borrows memory, `Bit 1` = owns `~self()` destructor):
  - **`0b00` `IValue`:** Trivial `memcpy` value types (`i64`, `f64`, `Vec3`, `array<T, N>`, `RangeWindow<T>`, `Dual<T, G>`), eligible for zero-copy `.pchor` cues and `as view` casts.
  - **`0b01` `IReference`:** Borrowed zero-copy windows (`slice<T>`, `stride<T>`, `str`, `SliceView<T>`, `OffsetLens<T>`); lexical escape verified by `selfhost/perceus/escape.pilot` (`ESCAPE001`–`ESCAPE003`).
  - **`0b10` `IObject`:** Owned heap/resource handles with `~self()` (`SliceBuffer<T>`, `List<T>`, `Signal<T>`, `EffectPlan<T>`); Perceus move-by-default, exact last-use `Drop`, and same-size `DropReuse` in-place recycling (`selfhost/perceus/arc.pilot`).
  - **`0b11` `IGuard`:** Scoped RAII guards (`MutexGuard<T>`, `ArenaScope`, `TxLease`) combining borrowed references with strict reverse-declaration **LIFO `ScopeExitDrop`**.

- **4-Tier `MethodTable`, `*Bundle` & `*Facet` Composition:**
  Replaces fragile OOP inheritance chains with deterministic 4-tier dispatch: Tier 1 (`struct` / `partial struct`) > Tier 2 (field-bearing `*Bundle` or 0-field `*Facet` splices with `coalesce`, `with`, `without`) > Tier 3 (sub-`interface` default) > Tier 4 (base `interface` default), alongside file-wide and lexically scoped `ext` / `use ext` extensions.

- **6 Homoiconic Call-Quoting Modes (Zero-Whitespace `(` Lookahead):**
  - `fn!(a, b)` $\rightarrow$ `CallExpr<TReturn>` (`ShallowQuote` typed AST call descriptor)
  - `fn!!(~a, b)` $\rightarrow$ `ast<TReturn>` (`DeepSplice` with `~` compile-time unquoting)
  - `fn!&(a, b)` $\rightarrow$ `frame<FnSig>` (`AbiFrame` zero-marshalling `#[repr(C)]` call struct)
  - `fn!^(a, b)` $\rightarrow$ `Signal<TReturn>` (`ReactiveSignal` glitch-free topological push-pull node)
  - `fn!@(a, b)` $\rightarrow$ `Dual<TReturn, TGrad>` (`SymbolicAdjoint` compile-time CSE forward/reverse derivative)
  - `fn!?(a, b)` $\rightarrow$ `EffectPlan<TReturn>` (`EffectDryRun` Copy-on-Write transactional sandbox)

- **Stage-3 Fixed-Point Self-Hosting & Multi-Target Codegen (`selfhost/`):**
  `selfhost/pilotc.pilot` compiles itself to both native C11 (`clang`) and `.pilc` v3 bytecode with bit-for-bit **Stage-2 == Stage-3 SHA-256 parity**, backed by pure-Pilot emitters for **C11, LLVM IR, WASM (`WASI`), x86_64/ARM64 ASM, 8 GPU backends (`PTX`, `AMDGPU`, `SPIR-V`, `WGSL`, `MSL`, `HLSL`, `CUDA C++`, `HIP C++`), 7 bidirectional transpilers (`C`, `C++`, `C#`, `Rust`, `Python`, `JS`, `Lua`)**, lossless Red/Green CST & Wadler-Lindig formatting, resident incremental LSP, and `VmHook`-stepped DAP debugging.

---

## Canonical Syntax & Type Reference

### 1. Symbolic Type Rewrites (`stdlib/core/syntax/type_construct.pilot`)

| Symbolic Form | Canonical `lowercase` Type | Lattice | Structural Layout / Protocol Contract |
| :--- | :--- | :--- | :--- |
| `T[]` | `slice<T>` | `0b01` | `{ ptr<T>, usize }` (16B contiguous view, `IReadOnlyList<T>`) |
| `T[N]` | `array<T, const N: usize>` | `0b00` | Inline `N * sizeof(T)` bytes (`IList<T>`) |
| `T[..]` | `stride<T>` | `0b01` | `{ ptr<T>, usize, isize }` (24B strided lens) |
| `T@N` | `simd<T, const N: usize>` | `0b00` | Hardware SIMD vector register (`IValue`) |
| `T*` | `ptr<T>` | `0b01` | Raw 8-byte pointer (`IIndexGet<usize, T>`, `IIndexSet<usize, T>`) |
| `&T` / `&var T` | `ref<T>` / `var ref<T>` | `0b01` | Lexical immutable / mutable borrow |
| `~T` | `iso<T>` | `0b10` | Isolated Perceus heap/cue transfer handle |
| `T?` | `option<T>` | `0b00*` | Niche-unboxed optional (`ITryOperator`, `ICoalesceOperator`) |
| `T ! E` | `result<T, E>` | `0b00*` | Values-first must-use outcome (`ITryOperator`) |
| `(A, B) => R` | `fn<(A, B), R>` | `0b01` | Closure view (`{ ptr<void>, ptr<void> }`) |
| `a ..< b` | `range<T>` | `0b00` | Half-open `[a, b)` (`IRangeOperator<T>`) |
| `a .. b` / `a ..= b` | `span<T>` | `0b00` | Inclusive `[a, b]` (`ISpanOperator<T>`) |
| `a .. b : s` | `step<T>` | `0b00` | Stepped progression (`IStepOperator<T>`) |
| `fn!&(a, b)` | `frame<TSig>` | `0b00` | Zero-marshalling `#[repr(C)]` C-ABI call frame |

### 2. Distributive Headers, Implicit Properties & `[...]` Attribute Pipelines

```pilot
[EnforceCoreNaming]
pub pkg Pilot.Core.Showcase;

pub fn distributive_demo() {
    let List<i32> {
        primary = [1, 2, 3,],
        backup  = [4, 5, 6,],
    };

    var Vec3<f64> {
        origin = (0.0, 0.0, 0.0),
        target = (9.0, 8.0, 7.0,),
    };
}

pub struct CombatUnit : IValue {
    pub:
        base_atk:  f32 = 50.0;
        crit_mult: f32 = 1.5;

        // Implicit property (`field` backing slot + `value` setter parameter):
        [Clamp(min: 0.0, max: 1000.0, weight: 100)]
        hp: f32 = 100.0 {
            get => field;
            set => { field = value; }
        }

        [Formula((u) => u.base_atk * u.crit_mult)]
        effective_atk: f32 { get; }

        [TraceSpan(name: "apply_damage", weight: 50)]
        var fn apply_damage(dmg: f32) {
            if dmg >= 0.0 {
                .hp = .hp - dmg;
            }
        }
}
```

### 3. Values-First Errors (`T ! E`), `guard`, `switch` & 3-Tier Dynamic `for` Loops

```pilot
pub fn scale_telemetry(samples: i64[]) -> i64[] ! str {
    guard samples.len() > 0 else {
        return err("empty sample window");
    }

    // Expression-oriented 3-tier `for` loop yielding a contiguous `SliceBuffer<i64>`:
    let scaled: i64[] = for i in 0 ..< samples.len() {
        let raw = samples[i];
        yield switch raw {
            case 0:         1;
            case (1 .. 10): raw * 10;
            default:        raw;
        };
    };
    return ok(scaled);
}
```

### 4. Homoiconic Call-Quoting (`!`, `!!`, `!&`, `!^`, `!@`, `!?`)

```pilot
pub pure fn energy(mass: f64, vel: f64) -> f64 => 0.5 * mass * vel * vel;

pub fn projections_demo(m: f64, v: f64) {
    let shallow_ast = energy!(m, v);     // CallExpr<f64>  (ShallowQuote)
    let deep_ast    = energy!!(~m, v);   // ast<f64>       (DeepSplice with `~m` unquoted)
    let abi_frame   = energy!&(m, v);    // frame<...>     (#[repr(C)] zero-marshalling struct)
    let live_sig    = energy!^(m, v);    // Signal<f64>    (Glitch-free push-pull reactive node)
    let dual_grad   = energy!@(m, v);    // Dual<f64, f64> (Compile-time symbolic derivative)
    let dry_run     = energy!?(m, v);    // EffectPlan<f64>(Copy-on-Write transactional sandbox)
}
```

---

## Repository Structure

```plaintext
Pilot/
├── crates/                      # Irreducible 14-Token Name-Blind Hardware & Physics Engine (12 crates)
│   ├── pilot-arena/             # 64KB slab bump allocator & typed generational arenas
│   ├── pilot-span/              # Zero-copy SIMD SWAR byte spans & LineIndex
│   ├── pilot-intern/            # Lock-free symbol & `:ns/kw` interner (14 seed keywords only)
│   ├── pilot-ast/               # Packed 16-byte `AstNode` & `AstArena`
│   ├── pilot-lexer/             # 14-token SIMD lexer, two-way dot, 6 call-quote modes & `bracket_links`
│   ├── pilot-parser/            # 14-token seed recursive-descent + Pratt parser & `ParserTables`
│   ├── pilot-analysis/          # Hardware layout physics (`LayoutTable`), `PhysicsRegistry` & bootstrap bridge
│   ├── pilot-rt/                # Zero-GC C-ABI runtime (`PilotVal` 14B SSO, OS `getrandom`, sockets, `dlopen`, threads)
│   ├── pilot-vm/                # 32-bit register `PilVM` v3, `VmHook` & `.pilc` v3 (`PIL3`) binary loader
│   ├── pilot-codegen/           # System toolchain link driver (`BuildDriver` -> `selfhost/codegen/*`)
│   ├── pilot-ffi/               # Minimal C-ABI host shim (`<= 180` LOC -> `selfhost/ffi_bindgen.pilot`)
│   └── pilot-cli/               # Thin `.pilc` & self-hosted CLI bootstrapper (`<= 100` LOC)
├── stdlib/                      # Pure Pilot Self-Genesis & Standard Library
│   ├── core/                    # 4-Stage Self-Genesis (`bootstrap`, `core`, `operator`, `mem`, `option`, `result`,
│   │   ├── primitives/          # 39 `[Lang]` layout-bound primitives (`i8`..`i128`, `f16`..`f128`, `slice`, `simd`, `frame`, ...)
│   │   ├── operators/           # 19 algebraic & structural operator contracts (`ITryOperator`, `ISplatOperator`, ...)
│   │   └── syntax/              # 16 homoiconic `syntax` & `syntax type_construct` modules (`for`, `switch`, `guard`, ...)
│   ├── crypto/                  # Pure Pilot SHA-256, SHA-512, BLAKE3, HMAC-SHA256 & RFC 8032 Ed25519
│   ├── db/                      # Relational SQL/B-Tree engine, PostgreSQL v3 & MySQL binary wire protocols
│   ├── http/                    # HTTP/1.1 client/server over `pilot-rt` raw TCP sockets
│   ├── polyglot/                # Bounds-checked SHM views, JSON marshaling & Python/Node/C bridges
│   ├── pkg/                     # SPDX license expression parser & compatibility gate (`licensing.pilot`)
│   └── ...                      # 35+ standard library packages (`collections`, `asyncio`, `realtime`, `contracts`, `numpy`, ...)
├── selfhost/                    # Self-Hosted Pilot Compiler, Backends & Developer Toolchain
│   ├── pilotc.pilot             # Stage-3 fixed-point compiler driver (Stage-2 == Stage-3 parity)
│   ├── lexer.pilot / parser.pilot / ast.pilot / preprocessor.pilot
│   ├── analyzer.pilot / attr_weaver.pilot / diagnostics.pilot / ffi_bindgen.pilot
│   ├── cst/ & fmt/              # Lossless Red/Green CST, O(1) bracket recovery & Wadler-Lindig formatter
│   ├── perceus/                 # 2-bit lattice Perceus ARC (`Dup`/`Drop`/`DropReuse`), escape & `[HardRealtime]` verifier
│   ├── vm/                      # `PilVM` v3 bytecode compiler & `.pilc` v3 debug-table binary writer
│   ├── codegen/                 # Layout-driven C11, LLVM IR, WASM, x86_64/ARM64 ASM & 8 GPU shader/kernel emitters
│   ├── transpile/               # 7-language emitters (`c`, `cpp`, `csharp`, `rust`, `python`, `js`, `lua`) & 6 ingesters
│   ├── lsp/ & dap/ & ide/       # Persistent incremental PilVM LSP, `VmHook`-stepped DAP & VS Code grammar generator
│   └── cli/ & pkg/ & perf/ & bench/ # Self-hosted CLI router, REPL, `pilot.toml`/archive/venv, profiler, WCET & optimizer
├── examples/                    # Canonical Tutorials (`tutorial/`), Concept Showcases (`showcase/`) & Benchmarks
├── scripts/                     # 4 Automated Architecture & Name-Blind Verification Gates (`gate1`–`gate4`)
└── vscode-pilot/                # Visual Studio Code extension generated via `selfhost/ide/vscode_gen.pilot`
```

---

## Build, Toolchain & Verification Commands

```bash
# Build the Pilot CLI binary
cargo build --release

# Execute a Pilot source file or precompiled `.pilc` v3 binary
./target/release/pilot run examples/execution_demo.pilot

# Verify syntax, 2-bit hardware lattice, clearance rings & Perceus contracts
./target/release/pilot check examples/overview.pilot

# Compile source to `.pilc` v3 bytecode or native AOT executable
./target/release/pilot compile examples/execution_demo.pilot -o execution_demo.pilc
./target/release/pilot build examples/execution_demo.pilot

# Bidirectional source transpilation (`c`, `cpp`, `csharp`, `rust`, `python`, `js`, `lua`)
./target/release/pilot --emit rust app.pilot -o app.rs
./target/release/pilot --from python lib.py -o lib.pilot

# Launch resident PilVM Language Server, stepped DAP server, or sync VS Code grammar
./target/release/pilot lsp
./target/release/pilot debug app.pilot
./target/release/pilot ide-sync

# Run all interactive concept showcases (`examples/showcase/`)
bash examples/showcase/run_all.sh

# Run the 4 Name-Blind & Rust Contraction Architecture Gates
python3 scripts/gate1_check.py && python3 scripts/gate2_check.py
python3 scripts/gate3_check.py && python3 scripts/gate4_selfhost_contraction.py

# Run all workspace unit tests & the simultaneous 44-test Milestone 50–61 Mega Gate
cargo test --workspace
cargo test -p pilot-analysis --test mega_all_milestones_simultaneous_gate
```
