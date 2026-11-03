# AGENTS.md — Pilot Language AI Coding Guidelines

## 1. Implementation Integrity (Zero Shortcuts)
- **No Hardcoded Answers:** Never return static, pre-computed, or dataset-specific values to make a test pass. Implement the complete underlying algorithm.
- **No Elided Logic:** Do not use `# TODO`, `pass`, `...`, `unimplemented!()`, or placeholder comments. Every generated function, branch, and edge case must be fully realized.
- **No Silent Fallbacks:** Do not swallow errors or return dummy default values on failure. Handle all states explicitly.
- **Pilot Code Over Others:** When there are no performance disadvantages to implementing code in Pilot over other languages (including the underlying Rust engine), implement the code in Pilot. If the code can be made into a useful library, package it in the stdlib.

## 2. Type System & Constrained Generics
- **Ban Untyped Containers:** Never use `Object`, `Any`, `void*`, or untyped dictionaries to bypass the type checker.
- **Prefer Constrained Generics:** Use parametric polymorphism bounded by explicit structural or trait constraints so monomorphization and static dispatch remain intact. Preserve `sizeof(T)` byte-width invariants and zero-cost monomorphization across all operator and interface bounds.

```pilot
// BAD: Type erasure and runtime casting
def process_nodes(items: [Object]) -> Object {
    return items[0]
}

// GOOD: Constrained generic preserving static dispatch
def process_nodes[T: AstNode + Serializable](items: Slice[T]) -> Result[T, ParseError] {
    if len(items) == 0 {
        return Err(ParseError.EmptyInput)
    }
    return Ok(items[0])
}
```

## 3. Disambiguation, Lexing & Method Call Rules

### 3.1 Two-Way Dot Classification (`PrefixDot` vs. `InfixDot`)
Inside type methods, `.field` is shorthand for `self.field`. Dot tokens are classified strictly at the lexer level:
- **`InfixDot` (`.`):** Preceded immediately (no intervening whitespace) by an identifier, numeric literal, `)`, `]`, or `}`. Valid ONLY in infix member access (`obj.field`, `tuple.0`).
- **`PrefixDot` (`.`):** Preceded by whitespace, newline/indentation, `;`, `(`, `[`, `{`, `,`, or `:` (e.g. `.reset()`, `0.. .count`, `[*.. .tree]`). Valid ONLY in prefix position; desugars directly to `self.<ident>`.
- **Adjacent Dot Rejection:** Never place dots directly adjacent without whitespace (e.g. `0...count` or `*...tree`). Always insert a space before prefix dot (`0.. .count`, `*.. .tree`). The lexer emits a clear diagnostic if adjacent dots are encountered.

### 3.2 Numeric Literals & Statement Termination
- **Leading-Zero Floats Only:** Floating-point literals MUST begin with an ASCII digit (`0-9`), followed by `.` and a digit (`0.5`). Bare `.5` is rejected at compile time with `Help: prefix float literals with 0 ('0.5')` to guarantee `.0` in `.tuple.0` never lexes as a float.
- **Semicolon `;` & Multiline Chaining:** A semicolon explicitly terminates a statement. A leading `.method()` on the subsequent line parses as `self.method()`. Without a terminating semicolon, an indented `.method()` on the next line continues the preceding expression as an infix call chain (`sender.wake()\n .with_timeout(50);`).
- **Contextual Enum Variant Resolution:** Enum variants must be referenced either explicitly (`Status.Active` or `Status::Active`) or via target-typed leading double-colon (`::Active`). Bare `.Active` is strictly prohibited for enums because leading dot `.ident` is reserved exclusively for `self.ident`.

### 3.3 Splat & Recursive Flattening Tokens (`*`, `**`, `*..`)
Tokenized via maximal munch:
- **`*` (Positional Splat):** 1-level slice rest pattern or positional expansion (`let [head, *payload, crc] = buf;`, `fn(*args)`).
- **`**` (Structural Splat):** 1-level compile-time named/structural field splat and deconstruction (`CpuState(pc: next_pc, **curr)`). Monomorphized statically by field offsets—zero runtime heap dictionaries.
- **`*..` (Deep Flatten):** Complete recursive deep-flatten to 1D leaf elements (`[*..nested_arr]`, `for leaf in *..tree:`). Constant-folded at compile time for literal structures; invokes `op flat self -> LeafT` for dynamic containers.

## 4. C#-Style Methods & First-Class `self` / `op` Grammar (Dunder-Free)
All Python-style dunder (`__...__`) magic methods are strictly prohibited. The compiler emits explicit diagnostics pointing to first-class `self` or `op` replacements. Regular method parameter lists have an implicit receiver; Python-style `(self, ...)` parameters are forbidden.

### 4.1 Method Signatures (`def`, `mut def`, `static def`)
- **`def foo(a: i32) -> R`:** Read-only instance method. Passes `*const Self` as hidden parameter 0. Fields can be accessed via `.field`, `self.field`, or bare `field` (unless shadowed by local variables/parameters).
- **`mut def foo(a: i32)`:** Mutating instance method. Passes `*mut Self` as hidden parameter 0.
- **`static def foo(a: i32) -> R`:** Associated function with no receiver. Referencing `self` or `.field` inside `static def` triggers compile-time error `STATIC001`.

### 4.2 Lifecycle & Accessor Forms
- **Constructor:** `self(params...)` (replaces `__init__` / `__new__`). Definite-assignment initialization block returning the initialized stack/arena value. Supports constructor chaining (`self(x: i32) -> self(x, 0)`).
- **Destructor:** `~self` (replaces `__del__`). Deterministic RAII cleanup emitted at scope exit.
- **Scoped Context:** `scope self { enter: ... exit: ... }` (replaces `__enter__` / `__exit__`). Deterministic resource acquisition and teardown.
- **Indexer:** `self[params...] -> T` or `self[params...] -> ref T` (replaces `__getitem__`, `__setitem__`, `__missing__`). Accessor clauses: `get:`, `set(val: T):`, and optional `default:`.
- **Dynamic Member Dispatch:** `self.(name: str) -> T` (replaces `__getattr__` / `__setattr__`). Accessor clauses: `get:` and `set(val: T):`.

### 4.3 Canonical `op` Operator and Protocol Reference Table

| Capability | Pilot Syntax | Replaces | Notes |
| :--- | :--- | :--- | :--- |
| **Binary Left** | `op self <binop> (rhs: R) -> Out` | `__add__`, `__mul__`, etc. | Standard left-hand instance operator |
| **Binary Right** | `op (lhs: L) <binop> self -> Out` | `__radd__`, `__rmul__`, etc. | Commutative / asymmetric LHS |
| **Compound Mut** | `op mut self <binop>= (rhs: R)` | `__iadd__`, `__imul__`, etc. | Mutates `self` in-place; zero allocation |
| **Unary Prefix** | `op <unop> self -> Out` | `__neg__`, `__invert__` | Prefix `-self`, `~self`, `!self` |
| **Equality** | `op self == (rhs: R) -> bool` | `__eq__`, `__ne__` | Auto-synthesizes `!=` unless overridden |
| **Functor Call** | `op self(params...) -> Out` | `__call__` | Distinct from constructor `self(...)` via `op` |
| **Membership** | `op (item: T) in self -> bool` | `__contains__` | Maps `x in container` directly |
| **Type Guard** | `op (val: T) is Self -> bool` | `__instancecheck__` | Compile-time / structural type check |
| **Implicit Cast** | `op self -> Target` | `__bool__`, safe coercions | Used in conditionals and widening |
| **Explicit Cast** | `op self as Target` | `__int__`, `__float__`, `__str__` | Invoked via `expr as Target` |
| **Formatted Str** | `op self as str(fmt: Slice[u8])` | `__format__` | Receives compile-time format slice |
| **Deconstruct** | `op match self -> (T1, T2, ...)` | `__match_args__` | Zero-alloc register unpacking in `match` |
| **Deep Flatten** | `op flat self -> LeafT` | Flatten recursion | Invoked by `*..self`; yields 1D elements |
| **Explicit Clone**| `op clone self -> Self` | `__copy__`, `__deepcopy__` | Explicit deep/arena clone boundary |
| **Hash** | `op hash self -> u64` | `__hash__` | Deterministic 64-bit structural hash |
| **Binary Wire** | `op pack self` / `op unpack self` | `__getstate__`, `__setstate__` | Direct C-ABI buffer serialization |
| **Sync Iterator** | `op for self -> T` (or `-> ref T`) | `__iter__`, `__next__` | Lowers to stack coroutine / loop fusion |
| **Async Future** | `op await self -> T` | `__await__` | Uses `yield_async`; stack state machine |
| **Async Stream** | `op async for self -> T` | `__aiter__`, `__anext__` | Async generator state machine |

```pilot
// Real Pilot example demonstrating the unified grammar
class TreeNode {
    var value: i64
    var children: [TreeNode]

    self(value: i64) {
        .value = value;
        .children = [];
    }

    self() -> self(0) {}

    ~self {
        .children.clear();
    }

    mut def add_child(node: TreeNode) {
        .children.append(node);
    }

    static def leaf(v: i64) -> TreeNode {
        return TreeNode(v);
    }

    op flat self -> i64 {
        yield .value;
        for c in .children {
            for leaf in *..c {
                yield leaf;
            }
        }
    }

    op for self -> i64 {
        yield .value;
        for c in .children {
            yield c.value;
        }
    }
}
```

### 4.4 Flat Sibling Property & Indexer Setters (Variation B)
Inside properties, `self[params...] -> T`, and `self.(name: str) -> T`, flat sibling accessor clauses reside at the same indentation level (never use unconstrained `_:` catch-alls):
- **`get:`** Returns canonical type `T`.
- **`default:`** Optional missing-key fallback inside `self[...]`.
- **`set:` (C#-Style Default):** Implicitly binds `value: T` matching the getter return type (`set: .data = value;`). Note: `.value` (`PrefixDot`) accesses `self.value`, while bare `value` references the incoming setter parameter.
- **`set(pattern: OtherType):` (Scoped Type/Deconstruct Overload):** Transforms or unpacks alternate incoming types locally on assignment (`set((a: u8, b: u8, c: u8, d: u8)):`, `set(Vec2(x, y)):`) without creating global implicit casts.
- **`set(CONST_LIT):` / `set(v: T) if cond:`:** Value-matched or guard-predicated setter branch.
- **`set[U: TraitBound](v: U):`:** Constrained generic setter monomorphized per call-site type `U`.
- **`set(iso v: OwnedT):`:** Reference-capability setter for zero-copy isolated ownership transfer.

```pilot
class ConfigMap {
    data: int

    property timeout: int {
        get:
            return .data;
        pub set:
            .data = value;
        pub set(scale: float):
            .data = (scale * 1000.0) as int;
        priv set(raw: (int, int)):
            .data = raw.0 + raw.1;
    }

    self[key: str] -> int {
        get:
            return .data;
        default:
            return 0;
        set:
            .data = value;
        set(iso token: str):
            .data = len(token);
    }
}
```

## 5. Two-Axis Visibility & Compile-Time Clearance Lattice (`lvl`)
Functions and methods are implicitly `private` by default. Prefix declarations and accessor clauses with modifiers from two orthogonal axes:
- **Axis 1 (10-Tier Spatial Visibility):**
  - `priv` (`private`): Enclosing type only (Functions/methods default to private).
  - `file` (`fileprivate`): Current source file only.
  - `priv prot` (`private protected`): Enclosing type + structural subtypes within current binary.
  - `prot` (`protected`): Enclosing type + structural subtypes/roles anywhere.
  - `pkg` (`package`): Current directory / module namespace.
  - `prot int` (`protected internal`): Current binary OR external structural subtypes.
  - `int` (`internal`): Current compiled binary / crate (Default for non-functions).
  - `pub` (`public`): Publicly callable; closed to external override/binding.
  - `open`: Publicly callable AND open for external `.pchor` role/extension binding.
  - `export`: Public + emits unmangled C-ABI symbol.
- **Axis 2 (Compile-Time Clearance Rings ? `lvl(N)` / `lvlN`):**
  - Support both `lvl(30)` / `lvl(Domain, 30)` and shorthand identifier `lvl30` (`^lvl([0-9]+)$` in modifier position).
  - Track active clearance `C` per lexical scope (default `C = 0`). Calling a target rated `lvl(N)` requires `C >= N` at compile time.
  - Elevate `C` either via transitive function/role declaration (`lvl30 def admin_op()`) or flow-typed guard blocks (`if lvl >= 30:`, `guard lvl >= 30 else: return`) with zero runtime overhead.
  - Emit diagnostic code `LVL001` when clearance is insufficient, and `VIS001` when an inaccessible private member is accessed from outside.

```pilot
lvl30 def secure_vault_access() {
    // Requires level 30 clearance
}

def dispatch_job(lvl: int) {
    if lvl >= 30:
        secure_vault_access(); // Permitted via flow-typed clearance elevation
}
```

## 6. Custom Language Deterrents (Anti-Hallucination)
- **Zero Syntax Hallucination:** Do not invent keywords, operators, decorators, or macro syntax borrowed from Python, Rust, or C# unless defined in the current workspace grammar.
- **Zero Stdlib Hallucination:** Do not assume standard library modules (e.g., `fs`, `net`, `json`, `math`) or built-in methods exist unless they are explicitly imported in context or declared in the project prelude. If a helper is missing, write it from primitives.
- **Verify Grammar Before Emitting:** Match the exact AST and token conventions found in neighboring `.pilot` and `.pchor` files.

## 7. Memory Layout, Slices & C-ABI FFI (`!&`)
- **Explicit FFI Layout:** When crossing the C-ABI boundary or using call quoting (`!&`), never assume compiler field reordering is safe. Use explicit alignment and fixed-width primitive types (`u8`, `i32`, `u64`, `usize`).
- **Slice Aliasing & Lifetimes:** Do not return pointers or slices referencing stack-allocated buffers that do not outlive the call frame. Keep ownership and buffer lifetimes explicit.
- **No Hidden Allocations:** Avoid implicit heap allocations inside hot loops or FFI trampolines; pass pre-allocated buffers or arena references where applicable.

## 8. Exhaustive Control Flow & State Handling
- **Exhaustive Matching:** Every tagged union, enum, or `Result` type must have all variants explicitly handled. Do not use catch-all wildcard branches (`_`) that mask unhandled future states.
- **Explicit Control Flow:** Avoid deep, implicit exception-style jumps. Propagate errors through return values and explicit state transitions.

## 9. Choreography (`.pchor`) vs. Core Implementation
- **Strict Separation of Concerns:** Keep execution contracts, node graph wiring, cues, and event routing inside `.pchor` choreography definitions.
- **Pure Implementation Nodes:** Keep concrete data transformations, bitwise math, and systems logic inside core Pilot modules. Never leak orchestration state machines into low-level compute functions.

```pilot
// .pchor Choreography Node
node DataIngestWorkflow {
    cue on_data_received(payload: RawPacket)
    contract requires len(payload.bytes) > 0

    route payload -> ValidateCrc -> DecryptFrame -> StoreRecord
}
```

## 10. Verification & Tooling Discipline
- **Compiler & Crucible First:** Always validate generated code against the `pilot` CLI compiler and existing crucible test suites before declaring a task complete.
- **Fix Root Causes in AST/Codegen:** If a compiler test fails, fix the AST lowering, type-checker constraint, or codegen rule—never patch or weaken the test oracle to hide a regression.
