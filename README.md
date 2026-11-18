# Pilot Programming Language

> **A systems and application language whose compiler, tools and standard library are being moved from a Rust bootstrap seed into Pilot itself.**

**Pilot** (`v1.1.51`) runs today on a Rust bootstrap seed: 8 crates of about 27,150 lines (`crates/`, capped by `paused_max_total_rust_loc()` in `selfhost/gate/manifest.pilot`). The seed reads `.pilot` source, and runs it on its register VM (PilVM). The checker, PilIR lowering and passes, the C11 emitter, the CLI commands and the standard library are written in Pilot (`selfhost/`, `stdlib/`). The plan is to shrink the seed to a small `.pilc` loader and VM (see [`docs/roadmap.md`](docs/roadmap.md)). Pilot does **not** compile itself yet: the bootstrap parity gate stops at the registered seam `bootstrap.stage_compile`.

**What is real today and what is design.** This README and [`VISION.md`](VISION.md) describe the target design. What actually runs, per engine, is recorded in [`docs/keyword_matrix.md`](docs/keyword_matrix.md), [`docs/concept_matrix.md`](docs/concept_matrix.md) and `selfhost/gate/ledger/wiring.pilot`. When this page and those files disagree, the files win. Every program is checked on three engines: the seed VM (`pilot test`), the PilIR interpreter (`pilot test --ir`) and generated C (`pilot test --emit c`).

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
  - **`0b10` `IObject`:** Owned heap/resource handles with `~self()` (`SliceBuffer<T>`, `List<T>`, `Signal<T>`, `EffectPlan<T>`); Perceus move-by-default and exact last-use drops (`selfhost/check/drops.pilot`, `selfhost/ir/pass_perceus.pilot`); same-size `DropReuse` in-place recycling is still to come.
  - **`0b11` `IGuard`:** Scoped RAII guards (`MutexGuard<T>`, `ArenaScope`, `TxLease`) combining borrowed references with strict reverse-declaration **LIFO `ScopeExitDrop`**.

- **4-Tier `MethodTable`, `*Bundle` & `*Facet` Composition:**
  Replaces fragile OOP inheritance chains with deterministic 4-tier dispatch: Tier 1 (`struct` / `partial struct`) > Tier 2 (field-bearing `*Bundle` or 0-field `*Facet` splices with `coalesce`, `with`, `without`) > Tier 3 (sub-`interface` default) > Tier 4 (base `interface` default), alongside file-wide and lexically scoped `ext` / `use ext` extensions.

- **6 Homoiconic Call-Quoting Modes & Pilot-First Mutable AST Arena (`selfhost/ast.pilot`):**
  - `fn!(a, b)` $\rightarrow$ `CallExpr<TReturn>` (`ShallowQuote` typed AST call descriptor)
  - `fn!!(~a, b)` $\rightarrow$ `ast<TReturn>` (`DeepSplice` with `~` compile-time unquoting)
  - `fn!&(a, b)` $\rightarrow$ `frame<FnSig>` (`AbiFrame` zero-marshalling `#[repr(C)]` call struct)
  - `fn!^(a, b)` $\rightarrow$ `Signal<TReturn>` (`ReactiveSignal` glitch-free topological push-pull node)
  - `fn!@(a, b)` $\rightarrow$ `Dual<TReturn, TGrad>` (`SymbolicAdjoint` compile-time CSE forward/reverse derivative)
  - `fn!?(a, b)` $\rightarrow$ `EffectPlan<TReturn>` (`EffectDryRun` Copy-on-Write transactional sandbox)
  - Backed by a **Pilot-First Mutable AST Arena** (`AstNodeSlot` 16B `0b00 : IValue`, `AstSubtreeView` `0b01 : IReference`, `AstArenaBuffer` `0b10 : IObject` with Perceus `drop_reuse` free-list slot recycling, bidirectional parent links, `$hole` & variadic `$*splat` splicing, `~expr` compile-time unquoting, lexical hygiene alpha-renaming, in-place constant folding).

- **Self-Hosting & Code Generation (`selfhost/`) — partly built:**
  Built and tested on every engine: PilIR lowering and passes (`selfhost/ir/`), C11 output (`selfhost/transpile/c_family.pilot` with `runtime/c/`; `pilot build`, `pilot test --emit c`), PilIR to `.pilc` bytecode (`selfhost/vm/`, `pilot compile`, `pilot test --via-ir`). Present but covering only a subset (check the matrices before relying on them): the source transpilers in `selfhost/transpile/` (`pilot emit <lang>`). There is no LLVM, WASM, assembly or GPU backend: the unused emitters once in `selfhost/codegen/` were deleted on 2026-11-18. Not built: the driver that compiles the compiler with itself (`selfhost/pilotc.pilot` stops at seam `bootstrap.stage_compile`), so there is no Stage-2 == Stage-3 parity yet.

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
| `a ..< b` | `range<T>` | `0b00` | Half-open `[a, b)` (`IRangeUntilOperator<T>`) |
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
├── AGENTS.md            # Rules for every contributor and AI coder (read first)
├── crates/              # The Rust bootstrap seed, 8 crates (~27,150 lines; rust.md maps every file)
│   ├── pilot-span/      # FileId and ByteSpan
│   ├── pilot-intern/    # String interner
│   ├── pilot-lexer/     # Runs the token table Pilot builds (stdlib/core/lex/toolchain.lex)
│   ├── pilot-ast/       # 16-byte AstNode arena
│   ├── pilot-parser/    # Seed parser
│   ├── pilot-analysis/  # Seed type inference (being replaced by selfhost/check/)
│   ├── pilot-vm/        # PilVM, .pilc loader, bytecode compiler, host ops (bridge.rs)
│   └── pilot-cli/       # The `pilot` executable: runs the Pilot CLI in selfhost/cli/
├── stdlib/              # Standard library in Pilot: core/ (language core), std/ (hosted), pilot/ (compiler as a library)
├── selfhost/            # Compiler and tools in Pilot: cli/, check/, ir/, vm/, transpile/, lsp/, dap/, gate/, ...
├── tests/conformance/   # The test corpus: <name>.pilot with .stdout/.stderr/.exit, run by `pilot test`
├── tools/               # Pilot tools (keyword_matrix, score) and capped.sh
├── docs/                # roadmap, keyword and concept matrices
└── vscode-pilot/        # VS Code extension
```

`pilot.md` maps every Pilot file and `rust.md` every Rust file.

---

## Build, Run & Verify

```bash
# Build the seed (debug build; agents and tests use target/debug/pilot)
cargo build -p pilot-cli

# Run, check, build to C, or compile to .pilc
./target/debug/pilot run examples/execution_demo.pilot
./target/debug/pilot check examples/overview.pilot
./target/debug/pilot build examples/execution_demo.pilot
./target/debug/pilot compile examples/execution_demo.pilot

# Transpile Pilot to another language, or ingest foreign source (subset; see the matrices)
./target/debug/pilot emit --help
./target/debug/pilot from --help

# Conformance tests on the three engines (add --changed while iterating)
./target/debug/pilot test
./target/debug/pilot test --ir
./target/debug/pilot test --emit c

# Architecture gates (airlock, hygiene, seams, keywords, rename litmus, bootstrap parity)
./target/debug/pilot gate --full

# Score a change against a base commit (tools/score; vetoes exit 1)
./target/debug/pilot run tools/score/main.pilot -- --base HEAD
```

Run long jobs under `tools/capped.sh SECONDS <cmd>` (AGENTS.md section 12). Rust crate tests (`cargo test -p <crate>`) matter only when a change touches the seed.
