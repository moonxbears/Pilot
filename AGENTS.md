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

## 3. Unified First-Class `self` & `op` Grammar (Dunder-Free)
All Python-style dunder (`__...__`) magic methods are strictly prohibited. The language grammar enforces a unified, first-class member system anchored on `self`, `~self`, `scope self`, and the `op` prefixtoken:

### 3.1 Lifecycle & Resource Management (`self`, `~self`, `scope self`)
- **Constructor:** `self(params...)` (replaces `__init__` / `__new__`). Treated as a definite-assignment initialization block returning the initialized stack/arena value. Supports constructor chaining via `self(params...) -> self(other_args...)`.
- **Destructor:** `~self` (replaces `__del__`). Deterministic RAII cleanup emitted at lexical scope exit.
- **Scoped Context Manager:** `scope self { enter: ... exit: ... }` (replaces `__enter__` / `__exit__`). Pairs acquisition and deterministic teardown in one unified construct.

### 3.2 Accessors & Dynamic Dispatch (`self[...]`, `self.(...)`)
- **Indexer:** `self[params...] -> T` or `self[params...] -> ref T` (replaces `__getitem__`, `__setitem__`, `__missing__`). Accessor clauses: `get:`, `set(val: T):`, and optional `default:`.
- **Dynamic Member Dispatch:** `self.(name: str) -> T` (replaces `__getattr__` / `__setattr__`). Accessor clauses: `get:`, `set(val: T):`.

### 3.3 Canonical `op` Operator and Protocol Reference Table

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
| **Explicit Clone**| `op clone self -> Self` | `__copy__`, `__deepcopy__` | Explicit deep/arena clone boundary |
| **Hash** | `op hash self -> u64` | `__hash__` | Deterministic 64-bit structural hash |
| **Binary Wire** | `op pack self` / `op unpack self` | `__getstate__`, `__setstate__` | Direct C-ABI buffer serialization |
| **Sync Iterator** | `op for self -> T` (or `-> ref T`) | `__iter__`, `__next__` | Lowers to stack coroutine / loop fusion |
| **Async Future** | `op await self -> T` | `__await__` | Uses `yield_async`; stack state machine |
| **Async Stream** | `op async for self -> T` | `__aiter__`, `__anext__` | Async generator state machine |

```pilot
// Real Pilot example demonstrating the unified grammar
class VectorBuffer {
    var data: [i64]
    var capacity: usize

    self(capacity: usize) {
        self.capacity = capacity
        self.data = []
    }

    self() -> self(16) {}

    ~self {
        self.data.clear()
    }

    self[index: usize] -> i64 {
        get => self.data[index]
        set(val: i64) => { self.data[index] = val }
        default => 0
    }

    op self + (rhs: VectorBuffer) -> VectorBuffer {
        let result = VectorBuffer(self.capacity)
        for i in 0..<len(self.data) {
            result.data.append(self.data[i] + rhs.data[i])
        }
        return result
    }

    op clone self -> Self {
        let copy = VectorBuffer(self.capacity)
        copy.data = self.data.clone()
        return copy
    }

    op for self -> i64 {
        for item in self.data {
            yield item
        }
    }
}
```

## 4. Custom Language Deterrents (Anti-Hallucination)
- **Zero Syntax Hallucination:** Do not invent keywords, operators, decorators, or macro syntax borrowed from Python, Rust, or C# unless defined in the current workspace grammar.
- **Zero Stdlib Hallucination:** Do not assume standard library modules (e.g., `fs`, `net`, `json`, `math`) or built-in methods exist unless they are explicitly imported in context or declared in the project prelude. If a helper is missing, write it from primitives.
- **Verify Grammar Before Emitting:** Match the exact AST and token conventions found in neighboring `.pilot` and `.pchor` files.

## 5. Memory Layout, Slices & C-ABI FFI (`!&`)
- **Explicit FFI Layout:** When crossing the C-ABI boundary or using call quoting (`!&`), never assume compiler field reordering is safe. Use explicit alignment and fixed-width primitive types (`u8`, `i32`, `u64`, `usize`).
- **Slice Aliasing & Lifetimes:** Do not return pointers or slices referencing stack-allocated buffers that do not outlive the call frame. Keep ownership and buffer lifetimes explicit.
- **No Hidden Allocations:** Avoid implicit heap allocations inside hot loops or FFI trampolines; pass pre-allocated buffers or arena references where applicable.

## 6. Exhaustive Control Flow & State Handling
- **Exhaustive Matching:** Every tagged union, enum, or `Result` type must have all variants explicitly handled. Do not use catch-all wildcard branches (`_`) that mask unhandled future states.
- **Explicit Control Flow:** Avoid deep, implicit exception-style jumps. Propagate errors through return values and explicit state transitions.

## 7. Choreography (`.pchor`) vs. Core Implementation
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

## 8. Verification & Tooling Discipline
- **Compiler & Crucible First:** Always validate generated code against the `pilot` CLI compiler and existing crucible test suites before declaring a task complete.
- **Fix Root Causes in AST/Codegen:** If a compiler test fails, fix the AST lowering, type-checker constraint, or codegen rule—never patch or weaken the test oracle to hide a regression.
