
# AGENTS.md — Pilot Language AI Coding Guidelines

## 1. Implementation Integrity (Zero Shortcuts)
- **No Hardcoded Answers:** Never return static, pre-computed, or dataset-specific values to make a test pass. Implement the complete underlying algorithm.
- **No Elided Logic:** Do not use `# TODO`, `pass`, `...`, `unimplemented!()`, or placeholder comments. Every generated function, branch, and edge case must be fully realized.
- **No Silent Fallbacks:** Do not swallow errors or return dummy default values on failure. Handle all states explicitly.
- **Pilot Code Over Others:** When there are not performance disadvantages to implementing code in pilot over other languages (including the underlying Rust engine) then implement the code through pilot. If the code can be made into a useful library then package it as one in the stdlib.

## 2. Type System & Constrained Generics
- **Ban Untyped Containers:** Never use `Object`, `Any`, `void*`, or untyped dictionaries to bypass the type checker.
- **Prefer Constrained Generics:** Use parametric polymorphism bounded by explicit structural or trait constraints so monomorphization and static dispatch remain intact.

```python
# BAD: Type erasure and runtime casting
def process_nodes(items: list[Object]) -> Object:
    return items[0]

# GOOD: Constrained generic preserving static dispatch
def process_nodes[T: AstNode + Serializable](items: Slice[T]) -> Result[T, ParseError]:
    if items.len == 0:
        return Err(ParseError.EmptyInput)
    return Ok(items[0])

```

## 3. Custom Language Deterrents (Anti-Hallucination)

* **Zero Syntax Hallucination:** Do not invent keywords, operators, decorators, or macro syntax borrowed from Python, Rust, or C# unless defined in the current workspace grammar.
* **Zero Stdlib Hallucination:** Do not assume standard library modules (e.g., `fs`, `net`, `json`, `math`) or built-in methods exist unless they are explicitly imported in context or declared in the project prelude. If a helper is missing, write it from primitives.
* **Verify Grammar Before Emitting:** Match the exact AST and token conventions found in neighboring `.pilot` and `.pchor` files.

## 4. Memory Layout, Slices & C-ABI FFI (`!&`)

* **Explicit FFI Layout:** When crossing the C-ABI boundary or using call quoting (`!&`), never assume compiler field reordering is safe. Use explicit alignment and fixed-width primitive types (`u8`, `i32`, `u64`, `usize`).
* **Slice Aliasing & Lifetimes:** Do not return pointers or slices referencing stack-allocated buffers that do not outlive the call frame. Keep ownership and buffer lifetimes explicit.
* **No Hidden Allocations:** Avoid implicit heap allocations inside hot loops or FFI trampolines; pass pre-allocated buffers or arena references where applicable.

## 5. Exhaustive Control Flow & State Handling

* **Exhaustive Matching:** Every tagged union, enum, or `Result` type must have all variants explicitly handled. Do not use catch-all wildcard branches (`_`) that mask unhandled future states.
* **Explicit Control Flow:** Avoid deep, implicit exception-style jumps. Propagate errors through return values and explicit state transitions.

## 6. Choreography (`.pchor`) vs. Core Implementation

* **Strict Separation of Concerns:** Keep execution contracts, node graph wiring, cues, and event routing inside `.pchor` choreography definitions.
* **Pure Implementation Nodes:** Keep concrete data transformations, bitwise math, and systems logic inside core Pilot modules. Never leak orchestration state machines into low-level compute functions.

## 7. Verification & Tooling Discipline

* **Compiler & Crucible First:** Always validate generated code against the `pilot` CLI compiler and existing crucible test suites before declaring a task complete.
* **Fix Root Causes in AST/Codegen:** If a compiler test fails, fix the AST lowering, type-checker constraint, or codegen rule—never patch or weaken the test oracle to hide a regression.

```

---

**Context & Rationale for Added Deterrents**
* **Syntax & Stdlib Anti-Hallucination (Section 3):** Because LLMs have massive priors on Python, Rust, and C#, their most common failure mode in a custom language is silently importing nonexistent stdlib packages or blending foreign syntax when uncertain. Forcing the agent to build from visible primitives prevents phantom API calls.
* **C-ABI & Slice Discipline (Section 4):** Custom languages that interface with C or compile down to native/staged targets frequently break when an AI assumes garbage-collected reference semantics around slices, struct padding, or FFI call quoting (`!&`).
* **Choreography Separation (Section 6):** Calling out the boundary between `.pchor` blueprints and core logic stops the agent from entangling high-level event routing with low-level algorithmic code.

<ElicitationsGroup message="Ways to tailor this AGENTS.md further:">
  <Elicitation label="Replace pseudo-Python with real Pilot & .pchor syntax" query="Update the code examples in AGENTS.md to use actual Pilot and .pchor syntax conventions."/>
  <Elicitation label="Add a compiler contributor section for Rust/C# internals" query="Add a section to AGENTS.md with rules for modifying the Pilot compiler, LSP, and DAP internals."/>
  <Elicitation label="Include CLI build and crucible test runner commands" query="Add exact CLI commands for building Pilot and running the Crucible test suites to AGENTS.md."/>
</ElicitationsGroup>