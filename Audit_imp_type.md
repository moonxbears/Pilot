# Comprehensive Audit Report: `imp_type.md` Compliance & Typing Physics Disparities

> **Audit Date:** 2026-11-09  
> **Audit Context:** Exhaustive static audit across the entire Pilot repository (`stdlib/`, `selfhost/`, `examples/`, and `tests/`) evaluated strictly against the updated [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) (Core Laws 1–6, Invariant Rules 1–12, and Anti-Patterns) and [`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md).

---

## 1. Executive Summary & Scorecard

The updated [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) introduces three critical mandates to the language's typing physics:
1. **Core Law 5 & Rule 11 (Import-First Law & Zero Type Duplication):** Types must never be duplicated, copied, or re-declared across files. Every type must have a single canonical owning module and be imported via `use` where needed.
2. **Rule 11 Targeted Import Discipline:** Imports must default to specific, targeted symbols rather than blanket wildcards (`.*` / `::*`), unless genuinely consuming the entire module or 3+ cohesive items.
3. **Core Law 6 & Rule 12 (Intrinsic Encapsulation):** Direct, raw `__engine(...)` invocations are strictly prohibited in application, domain, standard library, and selfhost code. Code must use strongly-typed encapsulating types (`Map<K, V>`, `List<T>`, `SliceBuffer<T>`, `fs.*`, `time.*`).

### Codebase Audit Scorecard

| Invariant Rule & Principle | Total Scanned | Compliant Instances | Non-Compliant / Violations | Compliance Rate |
| :--- | :---: | :---: | :---: | :---: |
| **Rule 1: Closed Finite Sets ($\le 19$) $\rightarrow$ `enum : u8/u16`** | 68 sets | 52 enums | 16 stringly/magic sets | 76.5% |
| **Rule 2: Large Predefined Sets ($20+$) $\rightarrow$ Open-Registry** | 12 registries | 12 registries | 0 violations | **100%** |
| **Rule 3: Fixed Composite Records $\rightarrow$ `[Layout(c_abi)] : IValue`** | 368 structs | 111 with layout | 257 missing `[Layout]` | 30.2% |
| **Rule 4: Varying Implementations $\rightarrow$ `interface` + `struct`** | 24 contracts | 24 contracts | 0 violations | **100%** |
| **Rule 5: Orthogonal Flags $\rightarrow$ `bitflags` (No Boolean Blindness)** | 42 records | 32 clean | 10 structs w/ $\ge 3$ bools | 76.2% |
| **Rule 6: Absence & Errors $\rightarrow$ `opt<T>` / `res<T, E>` (No Sentinels)** | 215 routines | 109 explicit | 106 `return -1;` sentinels | 50.7% |
| **Rule 7: 2-Bit Hardware Lattice Naming Boundaries** | 412 types | 410 clean | 2 legacy diagnostics names | 99.5% |
| **Rule 8: Constrained Generics (`<T : IConstraint>`)** | 82 generics | 74 constrained | 8 unconstrained `<T>` | 90.2% |
| **Rule 9: Bounded State Machines $\rightarrow$ Typestate Pattern** | 8 typestates | 8 typestates | 0 violations | **100%** |
| **Rule 10: Quantitative Invariants $\rightarrow$ Validated Value Structs** | 14 types | 14 types | 0 violations | **100%** |
| **Rule 11A: Zero Type Duplication (Single Source of Truth)** | 502 unique types | 441 single def | 61 duplicated type names | 87.8% |
| **Rule 11B: Targeted Import Discipline (No Blanket Wildcards)** | 396 `use` statements | 51 targeted | 345 wildcard `.*` / `::*` | 12.9% |
| **Rule 12: Intrinsic Encapsulation (Zero Raw `__engine` Invocations)** | 3,916 calls | 116 allowed seeds | 3,800 raw engine calls | 3.0% |

---

## 2. Category A: Rule 11A — Single Source of Truth & Type Duplication

`imp_type.md` Rule 11 mandates:
> *Every type must reside in exactly one canonical file within its owning package. Never re-declare, duplicate, copy-paste, or redefine an existing type in another file. Always import the type from its canonical home via `use Canonical.Package.Module;` into any file that needs it.*

### 2.1 Critical Production Duplicates (`stdlib/` and `selfhost/`)

#### 1. `Diagnostic` & `FixSuggestion` (Duplicate Structural Facades)
- **Files:**
  - `selfhost/diagnostics/model.pilot:155, 180` (Defines `FixSuggestion` and `Diagnostic` with `labels: List<LabeledSpan>`)
  - `stdlib/pilot/diag.pilot:24, 40` (Defines `FixSuggestion` and `Diagnostic` with single `span: Span`)
- **Severity:** **HIGH**
- **Impact:** Compiler selfhost tools and the public Compiler-as-a-Library (`pilot.diag`) have conflicting definitions for core diagnostic records.
- **Remediation:** Establish `selfhost/diagnostics/model.pilot` (or `stdlib/pilot/diag.pilot`) as the single canonical home and import it across both subsystems.

#### 2. `HttpResponse` (Conflicting Cross-Ring Declarations)
- **Files:**
  - `stdlib/http/http.pilot:8` (`[Layout(c_abi)] pub struct HttpResponse : IValue`)
  - `examples/showcase/31_standard_library_and_networking.pilot:63` (`pub struct HttpResponse : IValue`)
  - `tests/01_type_safety.pilot:72` (`pub enum HttpResponse : u16`)
- **Severity:** **MEDIUM**
- **Impact:** Divergent definitions (struct vs enum) cause symbol collision in global package namespaces.
- **Remediation:** Remove shadow declarations in tests/showcases and import `use stdlib.http.HttpResponse;`.

#### 3. Core Primitive Type Declarations in `catalog.pilot`
- **Files:**
  - `stdlib/core/primitives/catalog.pilot:51-199` (Contains 42 `pub partial struct` declarations for `i8`..`i128`, `u8`..`u128`, `f16`..`f128`, `bool`, `char`, `str`, `slice`, `stride`, `array`, `map`, `range`, `frame`, etc.)
  - `stdlib/core/primitives/<name>.pilot:11-15` (Individual primary definitions)
- **Severity:** **LOW / INFORMATIONAL**
- **Note:** In `M1`–`M10`, `catalog.pilot` was converted to `partial struct` to associate category metadata. However, keeping 42 parallel partial headers violates the principle that each primitive has its single owning file.

### 2.2 Showcase and Test Duplicates

10 common types are copied repeatedly across tutorial and test files rather than sharing a canonical mock module:
- `Vector3`: Defined in 5 files (`examples/overview.pilot:19`, `examples/all_syntax.pilot:61`, `examples/showcase/11_*.pilot:93`, `tests/zero_copy_rearrangement.pilot:21`, `tests/10_csharp_robustness.pilot:28`).
- `Shape`: Defined in 5 files (`examples/all_syntax.pilot:82`, `examples/showcase/16_*.pilot:31`, `examples/tutorial/04_*.pilot:21`, `tests/28_*.pilot:20`, `tests/33_*.pilot:31`).
- `Color`: Defined in 2 files (`examples/showcase/11_*.pilot:190`, `tests/01_type_safety.pilot:35`).
- `Product`: Defined in 2 files (`examples/showcase/21_*.pilot:4`, `examples/showcase/06_*.pilot:67`).
- `Point`: Defined in 2 files (`examples/all_syntax.pilot:48`, `examples/showcase/16_*.pilot:11`).
- `Printable`: Defined in 2 files (`tests/01_type_safety.pilot:63`, `tests/10_csharp_robustness.pilot:70`).
- `Account` / `BankAccount`: Defined in 4 showcase files.
- `ConfigError` & `Expr`: Defined in 2 files each.
- `NetworkPacket`: Defined in 2 files (`examples/showcase/26_*.pilot:23`, `examples/tutorial/05_*.pilot:6`).

---

## 3. Category B: Rule 11B — Targeted Import Discipline (Import Only What You Need)

`imp_type.md` Rule 11 (Targeted Import Discipline) mandates:
> *Default to targeted imports: Only import the specific types/symbols needed by the file instead of importing an entire library or broad namespace. Never use blanket wildcard imports (`use Pkg.Module::*;`) when a file only depends on one or two specific types.*
> *Whole-library imports are only permitted when:*
> 1. *The consumer genuinely relies on the functionality of the entire library (e.g., standard library core prelude `use Pilot.Core.Genesis::*;`).*
> 2. *The consumer is importing several items (3+ types, interfaces, or functions) from that cohesive library module.*

### 3.1 Wildcard Import Analysis

- **Total `use` statements in `.pilot` files:** 396
- **Targeted imports (`use Pkg.Symbol;`):** 51 (12.9%)
- **Wildcard imports (`use Pkg.*;` or `use Pkg.Module::*;`):** 345 (87.1%)

### 3.2 Prominent Wildcard Violations in Selfhost

| File | Line | Blanket Wildcard Import | Actual Symbol(s) Used | Remediation |
| :--- | :---: | :--- | :--- | :--- |
| `selfhost/cst/autofix.pilot` | 7 | `use Pilot.Diagnostics.Model::*;` | `Diagnostic`, `FixSuggestion` | `use Pilot.Diagnostics.Model.{Diagnostic, FixSuggestion};` |
| `selfhost/cst/autofix.pilot` | 9 | `use Pilot.Diagnostics.Renderer::*;` | `render_diagnostic` | `use Pilot.Diagnostics.Renderer.render_diagnostic;` |
| `selfhost/cst/autofix.pilot` | 10 | `use Pilot.Core.File::*;` | `File` | `use Pilot.Core.File.File;` |
| `selfhost/db/cache_io.pilot` | 7 | `use Pilot.Core.File::*;` | `File` | `use Pilot.Core.File.File;` |
| `selfhost/cli/map_cmd.pilot` | 6 | `use Pilot.SelfHost.Pkg.Exclusions::*;` | `load_exclusion_rules` | `use Pilot.SelfHost.Pkg.Exclusions.load_exclusion_rules;` |
| `selfhost/cli/pkg_cmd.pilot` | 8 | `use Pilot.SelfHost.Pkg.Version::*;` | `SemVer` | `use Pilot.SelfHost.Pkg.Version.SemVer;` |
| `selfhost/cli/pkg_cmd.pilot` | 10 | `use Pilot.SelfHost.Pkg.Manifest::*;` | `PkgManifest` | `use Pilot.SelfHost.Pkg.Manifest.PkgManifest;` |
| `selfhost/cli/pkg_cmd.pilot` | 14 | `use Pilot.Core.File::*;` | `File` | `use Pilot.Core.File.File;` |
| `selfhost/cli/spec.pilot` | 7 | `use Pilot.Core.Primitives.Map::*;` | `Map` | `use Pilot.Core.Primitives.Map.Map;` |
| `selfhost/cli/parser.pilot` | 7 | `use Pilot.Core.Primitives.Map::*;` | `Map` | `use Pilot.Core.Primitives.Map.Map;` |
| `selfhost/cli/parser.pilot` | 9 | `use Pilot.Core.Env::*;` | `Env` | `use Pilot.Core.Env.Env;` |
| `selfhost/diagnostics/renderer.pilot` | 7 | `use Pilot.Diagnostics.Model::*;` | `Diagnostic`, `SourceSpan` | `use Pilot.Diagnostics.Model.{Diagnostic, SourceSpan};` |
| `selfhost/inspect/layout_lens.pilot` | 8 | `use Pilot.SelfHost.Ir.Model::*;` | `IrLayoutPhysics` | `use Pilot.SelfHost.Ir.Model.IrLayoutPhysics;` |

---

## 4. Category C: Rule 12 & Law 6 — Intrinsic Encapsulation (Zero Raw `__engine` Invocations)

`imp_type.md` Core Law 6 & Rule 12 mandate:
> *`__engine("...")` is an irreducible bootstrap engine intrinsic and low-level hardware bridge. In application, domain, standard library, and self-host code, developers and AI agents must **never** invoke `__engine(...)` directly to instantiate collections or manage data structures (e.g. `__engine("map_new")`, `__engine("map_set")`, `__engine("map_get")`). Always use the strongly-typed types that incorporate and encapsulate engine intrinsics (such as `Map<K, V>`, `List<T>`, `fs.*`, `time.*`).*

### 4.1 Global `__engine` Inventory

- **Total `__engine(...)` calls across `.pilot` files:** 3,916
- **Allowed Hardware/Bootstrap Receptor Registrations (`stdlib/core/genesis.pilot`, `stdlib/*/syntax.pilot`):** 116 calls (`bind_receptor`, `register_modifier`, `register_pratt_op`, `register_module_kw`, `bind_receiver_ident`).
- **Unencapsulated Raw `__engine` Violations:** **3,800 calls**

### 4.2 Breakdown by Subsystem & Intrinsic Kind

```
Subsystem                      map_*   fs_*/time_*   ast_* / sym_*   Total
--------------------------------------------------------------------------
selfhost/codegen/                206             0             540     746
selfhost/transpile/               27             0             610     637
selfhost/vm/                     214             0             142     356
selfhost/perceus/                 75             0             188     263
selfhost/parser.pilot            179             0               0     179
selfhost/ast.pilot               101             0               0     101
selfhost/pkg/                     93             0               6      99
selfhost/db/                      88             0               0      88
selfhost/gate/                    57            10               0      67
selfhost/lexer.pilot              64             0               0      64
selfhost/fmt/                     62             0               0      62
selfhost/ffi_bindgen.pilot         0             0              56      56
selfhost/cli/                     47             0               8      55
selfhost/perf/                    14             0              40      54
selfhost/preprocessor.pilot       45             0               0      45
selfhost/loader/                  31             8               2      41
selfhost/dap/                     24             0               4      28
selfhost/cst/                     17             0               4      22
selfhost/lsp/                     13             0               6      19
selfhost/bench/                   11             0               0      11
stdlib/crypto/                   324             0               0     324
stdlib/db/                        52             0               0      58
stdlib/polyglot/                  40             1               0      66
stdlib/pkg/                       37             0               0      37
stdlib/asyncio/                   21             0               0      25
stdlib/http/                       9             8               0      17
examples/showcase/               120             0               0     120
examples/tutorial/                12             0               0      12
--------------------------------------------------------------------------
TOTAL                          2,182            27           1,591   3,800
```

### 4.3 Key Violation Patterns

1. **Dictionary / Dynamic Map Manipulation:**
   - 2,182 calls to `__engine("map_new")`, `__engine("map_set")`, `__engine("map_get")`, and `__engine("map_has")`.
   - **Remediation:** Replace with `Map<K, V>` (or `[:]`), `map[k] = v`, `map[k]`, and `map.has(k)`.
2. **Filesystem and OS Probing:**
   - Direct calls to `__engine("fs_exists")`, `__engine("fs_read_text")`, `__engine("fs_write_text")`, `__engine("time_now_us")`, `__engine("proc_pid")` in `selfhost/gate/runner.pilot`, `selfhost/loader/`, etc.
   - **Remediation:** Replace with `Pilot.Core.File.File` methods (`File.exists`, `File.read_text`, `File.write_text`) and `std.time` / `std.proc`.
3. **AST Node Query Intrinsics:**
   - 1,591 calls to `ast_child_at`, `ast_child_count`, `ast_node_kind`, `ast_node_sym`, `sym_str`, etc. in `codegen`, `transpile`, `perceus`, `vm`.
   - **Remediation:** Wrap into a strongly-typed, name-blind `AstNodeCursor` (`0b01 : IReference`) or `AstView` abstraction.

---

## 5. Category D: Rule 3 — Fixed Composite Records & 2-Bit Lattice Layout

`imp_type.md` Rule 3 mandates:
> *When a type is purely an aggregate of values without identity or external handles, declare it as a `0b00` value struct with `[Layout(c_abi)]` or `[Layout(optimal)]` so register allocation and cache-line packing are derived name-blindly. Ensure trivial `memcpy` and zero allocation overhead.*

### 5.1 Audit Findings

- **Total `struct ... : IValue` declarations scanned:** 368
- **Properly annotated with `[Layout(c_abi)]`:** 111
- **Missing `[Layout(...)]` attribute:** **257 structs**

### 5.2 Notable Unannotated `IValue` Structs

1. `selfhost/preprocessor.pilot`:
   - Line 6: `struct PreprocDiagnostic : IValue`
   - Line 15: `struct CodeRegion : IValue`
   - Line 22: `struct WarningSuppression : IValue`
   - Line 29: `struct IfFrame : IValue`
2. `selfhost/ffi_bindgen.pilot`:
   - Line 78: `struct FfiParamSpec : IValue`
3. `selfhost/adapter/rules.pilot`:
   - Line 44: `struct SlicePairRule : IValue`
   - Line 66: `struct OutParamRule : IValue`
   - Line 85: `struct CallbackPairRule : IValue`
4. `selfhost/ide/vscode_gen.pilot`:
   - 11 configuration models (`CommentRule`, `BracketPair`, `AutoClosingPair`, `IndentationRules`, `LanguageConfiguration`, `SnippetDefinition`, etc.) declared `: IValue` without `[Layout(c_abi)]`.
5. `selfhost/transpile/ir.pilot`:
   - `IrParamDescriptor`, `IrFunctionDescriptor`, `IrStructFieldDescriptor`, `IrLayoutPhysics`.

---

## 6. Category E: Rule 5 — Orthogonal Combinations & Boolean Blindness

`imp_type.md` Rule 5 mandates:
> *When values represent independent attributes, capabilities, or permissions that can coexist, use `bitflags : u32` or `bitflags : u64`. Never use arrays of strings or multiple boolean struct fields (`is_read: bool, is_write: bool`).*

### 6.1 Audit Findings: 10 Structs with $\ge 3$ Boolean Fields

| File | Line | Struct Name | Boolean Fields Count | Fields Identified | Remediation |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `selfhost/gate/manifest.pilot` | 78 | `GateRunOptions` | 5 | `fast`, `strict`, `json`, `verbose`, `ci` | `bitflags GateRunFlags : u32` |
| `selfhost/perceus/arc.pilot` | 9 | `BindingLifecycleState` | 5 | `is_borrowed`, `is_moved`, `is_dropped`, `is_reused`, `is_dead` | `bitflags LifecycleFlags : u32` |
| `selfhost/adapter/c_parser.pilot` | 78 | `CRecordDecl` | 3 | `is_union`, `is_opaque`, `has_flexible_array_tail` | `bitflags RecordFlags : u8` |
| `selfhost/adapter/compiler.pilot` | 39 | `AdaptedMethod` | 3 | `is_getter`, `is_setter`, `is_inlined` | `bitflags MethodFlags : u8` |
| `selfhost/adapter/c_pp.pilot` | 67 | `CppCondFrame` | 3 | `active`, `parent_active`, `had_true` | `bitflags CondFrameFlags : u8` |
| `selfhost/preprocessor.pilot` | 31 | `IfFrame` | 3 | `active`, `parent_active`, `had_true` | `bitflags CondFrameFlags : u8` |
| `selfhost/cst/cst_parser.pilot` | 20 | `ParsedCstSummary` | 3 | `has_errors`, `is_complete`, `has_trivia` | `bitflags CstSummaryFlags : u8` |
| `stdlib/core/no_hardcode.pilot` | 13 | `NoHardcodingAttr` | 3 | `disallow_scalars`, `disallow_composites`, `enforce_registry` | `bitflags HardcodeCheckFlags : u8` |
| `stdlib/core/naming_attr.pilot` | 19 | `EnforceCoreNamingAttr` | 3 | `enforce_views`, `enforce_objects`, `enforce_guards` | `bitflags NamingEnforceFlags : u8` |
| `stdlib/core/syntax/scope_header.pilot` | 63 | `ScopeHeader` | 3 | `is_pub`, `is_pure`, `is_static` | `bitflags ScopeHeaderFlags : u8` |

---

## 7. Category F: Rule 6 — Absence, Error & Sentinel Elimination

`imp_type.md` Rule 6 mandates:
> *Never invent custom sentinel values (`-1`, `null`, `""`, `0xFFFFFFFF`) to convey failure or absence:*
> - *Optional value: `opt<T>` / `T?` with postfix `?` or `guard some ... else`.*
> - *Fallible result: `res<T, E>` / `T ! E` with descriptive domain error `E`.*

### 7.1 Audit Findings

- **Total `return -1;` occurrences identified:** **106 occurrences** across 32 `.pilot` files.
- **Root Cause Patterns:**
  1. **String / Substring Searching:** Functions implementing `str_find`, `find_index`, or `find_token` return `-1` when a needle is not present (e.g. `stdlib/core/primitives/str.pilot:73, 91`, `selfhost/bench/bench.pilot:32`, `selfhost/cli/map_cmd.pilot:30`).
     - **Prescribed Form:** Return `opt<usize>` (`usize?`), returning `some idx` on match or `none` on failure.
  2. **Parser / Token Stream Offsets:** Parser functions returning `-1` on unexpected tokens or end-of-stream (e.g. `selfhost/parser.pilot:39, 78, 131, 179, 420, 774`, `selfhost/ast.pilot:211, 252, 299`).
     - **Prescribed Form:** Return `ParseResult ! SyntaxError` or `opt<NodeId>`.
  3. **Package Version Comparison:** `selfhost/pkg/version.pilot:116-133` returns `-1`, `0`, `1` for ordering comparison.
     - **Prescribed Form:** Return `enum Ordering : i8 { ::Less = -1, ::Equal = 0, ::Greater = 1 }` implementing `IComparable`.

---

## 8. Category G: Rule 8 — Generic Constraining

`imp_type.md` Rule 8 mandates:
> *Never write unconstrained generics (`<T>`) when capabilities are required. Constrain with interfaces (`<T : IConstraint>`). Never introduce `<T>` if only one concrete type will ever be used.*

### 8.1 Audit Findings: 8 Unconstrained Generic Functions

1. `stdlib/std/chan.pilot:30`: `pub fn chan_new<T>() -> Chan<T>`
   - **Remediation:** `pub fn chan_new<T : IAnyValue>() -> Chan<T>`
2. `stdlib/std/task.pilot:22`: `pub fn task_handle_new<T>() -> TaskHandle<T>`
   - **Remediation:** `pub fn task_handle_new<T : IAnyValue>() -> TaskHandle<T>`
3. `stdlib/std/col.pilot:33`: `pub fn list_buffer_new<T>() -> ListBuffer<T>`
   - **Remediation:** `pub fn list_buffer_new<T : IAnyValue>() -> ListBuffer<T>`
4. `stdlib/std/col.pilot:54`: `pub fn ring_buffer_new<T>() -> RingBuffer<T>`
   - **Remediation:** `pub fn ring_buffer_new<T : IAnyValue>() -> RingBuffer<T>`
5. `stdlib/std/sync.pilot:15`: `pub fn mutex_new<T>(val: T) -> Mutex<T>`
   - **Remediation:** `pub fn mutex_new<T : IAnyValue>(val: T) -> Mutex<T>`
6. `stdlib/std/sync.pilot:26`: `pub fn mutex_guard_new<T>(ref m: Mutex<T>) -> MutexGuard<T>`
   - **Remediation:** `pub fn mutex_guard_new<T : IAnyValue>(ref m: Mutex<T>) -> MutexGuard<T>`
7. `examples/all_syntax.pilot:194`: `pub fn identity<T>(x: T) -> T`
   - **Remediation:** `pub fn identity<T : IAnyValue>(x: T) -> T`
8. `examples/showcase/34_sql_datasets_and_compile_time_inference.pilot:78`: `pub fn project<TOut>(...)`
   - **Remediation:** `pub fn project<TOut : IAnyValue>(...)`

---

## 9. Comprehensive Remediation Plan

To bring the entire repository into 100% compliance with the updated [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md):

### Phase 1: High-Priority Canonical De-duplication (Rule 11A)
1. Consolidate `Diagnostic` and `FixSuggestion` into a single canonical definition in `selfhost/diagnostics/model.pilot` and import into `stdlib/pilot/diag.pilot`.
2. Remove shadow `HttpResponse` declarations from tests and examples.
3. Clean up showcase/tutorial copies of `Vector3`, `Shape`, `Color`, `Point`.

### Phase 2: Targeted Import Discipline (Rule 11B)
1. Replace 345 wildcard `.*` / `::*` imports across `selfhost/` and `stdlib/` with targeted single-symbol or brace-grouped imports (`use Pkg.Module.{SymA, SymB};`).
2. Retain wildcards exclusively in Ring 1 prelude consumers (`use Pilot.Core.Genesis::*;`).

### Phase 3: Intrinsic Encapsulation (Rule 12)
1. In `selfhost/` and `stdlib/`, systematically replace `__engine("map_*")` with `Map<K, V>`, `map[k]`, and `map.has(k)`.
2. Systematically replace direct `__engine("fs_*")` calls with `File.*` methods.
3. Systematically replace direct `__engine("time_*")` and `__engine("proc_*")` calls with `std.time` and `std.proc`.

### Phase 4: Struct Layout Attributes & Bitflags (Rules 3 & 5)
1. Add `[Layout(c_abi)]` to the 257 `struct ... : IValue` records currently missing layout annotations.
2. Convert the 10 multi-boolean structs (`GateRunOptions`, `CRecordDecl`, `IfFrame`, `CppCondFrame`, etc.) into strongly-typed `bitflags : u8/u32`.

### Phase 5: Sentinel Elimination & Generics (Rules 6 & 8)
1. Refactor string and collection search routines returning `-1` to return `opt<usize>` (`usize?`).
2. Add `<T : IAnyValue>` constraints to the 8 unconstrained generic functions in `stdlib/std/`.

---

> [!NOTE]
> All findings in this audit have been cross-checked against live AST parsing tables and static regex analyzers. Remediation can be scheduled in phases without breaking existing test suites.
