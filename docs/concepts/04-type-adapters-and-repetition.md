# Concept 04: First-Class Type Adapters & Scalar-to-Collection Repetition

[← Prev: Method Groups & Dispatch](./03-method-groups-and-dispatch.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Algebraic Classes & Typestates →](./05-algebraic-classes-and-typestates.md)

---

## 1. Concept Overview & Design Rationale

Pilot eliminates boilerplate data-plumbing code in two major areas:

1. **First-Class Type Adapters (`adapter FromType -> ToType`)**: Instead of scattering ad-hoc parsing functions across a codebase, you can register global, deterministic conversion rules between any two types (`str -> int`, `str -> bool`, `map -> User`, or `Celsius -> Fahrenheit`). Adapters are automatically consulted during typed variable assignments, typed function parameter binding, `as` casts, and structural dictionary-to-class hydration.
2. **Scalar-to-Collection Repeated Calls (`each` / `repeat` / `@vectorized` / `with auto_repeat`)**: Any function written for a single scalar value (`def square(x: int) -> int`) can be lifted to operate over collections (`list`, `tuple`, `set`, `range`) without writing wrapper loops—either at the declaration site (`def f(each x: int)`), at the call site (`f(each items)`, `f*(items)`), via method lifting (`list.append.each(items)`), or across an entire lexical block (`with auto_repeat { ... }`).

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `ADAPTER` | `adapter` | Declares a first-class type conversion block |
| `ARROW` | `->` | Separates source type and target type in `adapter Source -> Target` |
| `AS` | `as` | Explicit cast/conversion expression (`val as TargetType`) |
| `EACH`, `REPEAT` | `each`, `repeat` | Parameter-level, call-site, member-chain, or `with`-block repetition modifiers |
| `RANGE` | `..` | Call-site collection repetition / spread prefix (`fn(..items)`) |
| `STAR` | `*` | Vectorized broadcast invocation (`fn*(items)`) |
| `AT` | `@` | Decorator prefix for `@vectorized` / `@auto_repeat` functions |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// First-Class Type Adapter Declaration
adapter_decl
    : ADAPTER type_annotation ARROW type_annotation suite
    ;

// Explicit Cast / Adapter Application
cast_expr
    : pipe_expr (AS type_annotation)*
    ;

// Parameter-Level Repetition Modifier
param
    : (CONTEXT | EACH | REPEAT | PARAMS)? REF? IDENTIFIER (COLON type_annotation)? (ASSIGN expr)?
    ;

// Call-Site Repetition Argument Modifiers
argument
    : IDENTIFIER ASSIGN expr
    | (STAR | POWER | RANGE | EACH | REPEAT | REF) expr
    | expr (FOR exprlist IN testlist (IF expr)*)?
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 `PilotTypeAdapterRegistry` & Structural Hydration
[`PilotTypeAdapterRegistry.cs`](../../Runtime/Adapters/PilotTypeAdapterRegistry.cs) maintains a lookup table keyed by `(NormalizeTypeName(fromType), NormalizeTypeName(toType))`:
- **Built-in Default Adapters**: Registered automatically at startup for `(str -> int)`, `(str -> float)`, `(str -> bool)` (`"true"`, `"1"`, `"yes"`, `"y"`, `"on"` -> `true`; `"false"`, `"0"`, `"no"`, `"n"`, `"off"` -> `false`), `(int -> str)`, `(float -> str)`, `(bool -> str)`, `(int -> float)`, `(float -> int)`, `(list -> set)`, and `(set -> list)`.
- **Declarative `adapter A -> B` Blocks**: Executed by `VisitAdapter_decl`. Creates a synthetic single-parameter `PilotFunction` with parameter `self` and registers it in `Context.Engine.TypeAdapters`.
- **Automatic Consultation Points**:
  1. **Typed Variable Declarations (`let x: Target = expr`)**: If `expr`'s runtime type differs from `Target`, `CoerceToDeclaredType` calls `TypeAdapters.TryConvert` and falls back to structural `map -> Class` hydration.
  2. **Typed Function Parameters (`def f(x: Target)`)**: `BindParameters` automatically invokes matching adapters when an argument's runtime type does not match `Target`.
  3. **Explicit `expr as Target`**: `VisitCast_expr` runs user/built-in adapters, or hydrates a `Class` instance by matching `map` keys to class fields and properties.

### 4.2 Five Modes of Scalar-to-Collection Repetition
In [`PilotInterpreterVisitor.cs`](../../Runtime/Execution/PilotInterpreterVisitor.cs), `IsRepeatableCollection` recognizes `list`, `tuple`, `set`, and `range`. When repetition is triggered:
- If the function calls return non-`none` values, they are collected and returned as a new `list`.
- If all calls return `none` (e.g., `logger.append.each(items)`), `none` is returned.

| Mode | Syntax | How the Runtime Executes It |
| :--- | :--- | :--- |
| **1. Parameter Modifier** | `def tax(each price: float)` | Marks `PilotParameter.IsEach = true`. Passing a collection to `price` automatically loops `tax` over every element. Passing a scalar executes once normally. |
| **2. Call-Site Modifier** | `tax(each prices)` / `tax(repeat prices)` / `tax(..prices)` | Marks `EvaluatedArg.IsEach = true` at the call site, fanning out the call across every element of `prices` even if `tax` was declared as a normal scalar function. |
| **3. Member `.each` Chain** | `out_list.append.each(items)` | Accessing `.each` or `.repeat` on any callable returns a vectorized wrapper function (`CreateEachWrapper`) that invokes the underlying method once per item in the collection argument. |
| **4. Broadcast Splat / `@vectorized`** | `tax*(prices)` or `@vectorized def tax(p)` | Invokes the function element-wise across collection arguments (zipping multiple collections up to the shortest length while holding scalar arguments constant). |
| **5. Scoped `with auto_repeat`** | `with auto_repeat { ... }` | Increments `_autoRepeatDepth`. Inside the block, **any** function call where a collection is passed to a parameter typed with a non-collection annotation (`int`, `str`, `float`, etc.) automatically repeats element-wise. |

---

## 5. Available Functions, Methods & Properties

| Function / Member | Signature | Description |
| :--- | :--- | :--- |
| `register_adapter` | `register_adapter(from_type: str, to_type: str, fn: callable) -> none` | Registers a custom conversion function in `PilotTypeAdapterRegistry`. |
| `TypeAdapter.register` | `TypeAdapter.register(from_type: str, to_type: str, fn: callable) -> none` | Module-qualified alias for `register_adapter`. |
| `vectorize` | `vectorize(fn: callable) -> callable` | Returns a new function with `IsVectorized = true` (same as `@vectorized`). |
| `callable.each` / `.repeat` | `fn.each(collection) -> list \| none` | Property on any function or bound method that lifts it to iterate over a collection argument. |
| `auto_repeat` / `repeat` | `with auto_repeat { ... }` | Context sentinel enabling automatic scalar-to-collection lifting inside the `with` block. |

---

## 6. Complete Runnable Examples

See [`examples/showcase/04_type_adapters_and_repetition.pilot`](../../examples/showcase/04_type_adapters_and_repetition.pilot) for the full executable showcase.

```pilot
// 1. Declarative Type Adapter: str -> list (CSV splitter)
adapter str -> list {
    return self.split(",")
}

let tags: list = "alpha,beta,gamma"
assert(tags == ["alpha", "beta", "gamma"])

// 2. Structural Map -> Class Hydration
class ServerConfig {
    let host: str = "127.0.0.1"
    let port: int = 80
    let debug: bool = false
}

// Automatically hydrates ServerConfig AND applies built-in str->int / str->bool adapters!
let raw_env = {"host": "0.0.0.0", "port": "9090", "debug": "yes"}
let cfg: ServerConfig = raw_env
assert(cfg.host == "0.0.0.0" and cfg.port == 9090 and cfg.debug == true)

// 3. Parameter-level repetition ('each')
def format_tag(each tag: str) -> str {
    return f"#{tag.upper()}"
}
assert(format_tag("solo") == "#SOLO")
assert(format_tag(["dev", "ops"]) == ["#DEV", "#OPS"])

// 4. Call-site repetition, .each member chaining, and 'with auto_repeat'
def cube(n: int) -> int { return n * n * n }

assert(cube(each [2, 3, 4]) == [8, 27, 64])
assert(cube*([1, 2, 3]) == [1, 8, 27])

let sink = []
sink.append.each(["a", "b", "c"])
assert(sink == ["a", "b", "c"])

with auto_repeat {
    // 'cube' expects 'int', so passing a range or list automatically lifts it
    let cubes = cube(1..3)
    assert(cubes == [1, 8, 27])
}
```

---

[← Prev: Method Groups & Dispatch](./03-method-groups-and-dispatch.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Algebraic Classes & Typestates →](./05-algebraic-classes-and-typestates.md)
