# Concept 01: Types, Variables, Collections & Control Flow

[🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Guard, Defer & Exceptions →](./02-guard-defer-exceptions.md)

---

## 1. Concept Overview & Design Rationale

Pilot's core execution layer combines the approachable readability of Python and JavaScript with the type precision of Rust, C#, and TypeScript. Every value in Pilot is backed by a strongly structured runtime representation (`PilotValue` and `PilotValueKind`) while supporting gradual typing:

- **Optional & Gradual Type Annotations**: Variables and parameters can omit types (`let x = 10`), bind to exact hardware bit-widths (`let reg: u16 = 0xFF00`), or use algebraic union (`int | str`), intersection (`Readable & Writable`), and nullable (`User?`) types.
- **Universal Block Ergonomics**: Every control-flow construct accepts **either** C-style braces (`{ ... }`) **or** Python-style colon suites (`:`), even within the same file.
- **Fluent Data Pipelines**: First-class safe navigation (`?.`), Dart/Smalltalk-style object cascades (`..` and `?..` with `->` target capture), and functional pipeline operators (`|>` and `<|`) eliminate temporary intermediate variables.

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `LET`, `VAR`, `CONST` | `let`, `var`, `const` | Variable and constant binding keywords |
| `DYNAMIC`, `TYPE` | `dynamic`, `type` | Dynamic typing modifier and type alias declaration |
| `REF` | `ref` | Reference binding modifier for variables and arguments |
| `TYPE_INT`, `TYPE_UINT` | `int`, `i8`, `i16`, `i32`, `i64`, `uint`, `u8`, `u16`, `u32`, `u64` | Signed and unsigned integer type keywords |
| `TYPE_FLOAT`, `TYPE_DOUBLE`, `TYPE_DECIMAL`, `TYPE_COMPLEX` | `float`, `f8`, `f16`, `f32`, `f64`, `double`, `decimal`, `complex` | Floating-point, decimal, and complex number type keywords |
| `TYPE_BOOL`, `TYPE_CHAR`, `TYPE_STR`, `TYPE_SYMBOL` | `bool`, `char`, `str`, `string`, `symbol` | Primitive scalar and text type keywords |
| `NONE`, `NULL`, `UNDEFINED` | `None`, `none`, `null`, `undefined` | Null/empty value and type tokens |
| `TRUE`, `FALSE` | `True`, `true`, `False`, `false` | Boolean literals |
| `IF`, `ELIF`, `ELSE` | `if`, `elif`, `else` | Conditional branching and inline ternary expression tokens |
| `FOR`, `IN`, `WHILE` | `for`, `in`, `while` | Iteration loop keywords |
| `BREAK`, `CONTINUE`, `PASS` | `break`, `continue`, `pass` | Loop jump statements and no-op statement |
| `RANGE`, `QUESTION_DOT_DOT` | `..`, `?..` | Inclusive range (`0..5`), collection spread (`..items`), and object cascade (`obj..m()`) |
| `QUESTION`, `QUESTION_DOT`, `COALESCE`, `COALESCE_ASSIGN` | `?`, `?.`, `??`, `??=` | Nullable type suffix, safe navigation, null-coalescing, and null-coalescing assignment |
| `PIPE_FORWARD`, `PIPE_BACKWARD` | `|>`, `<|` | Forward and backward functional pipeline operators |
| `ARROW` | `->` | Cascade target capture (`.. -> saved`) and return type separator |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// Variable, Constant & Type Alias Declarations
var_decl
    : (LET | VAR | CONST) REF? IDENTIFIER (COLON type_annotation)? (ASSIGN testlist_star_expr)?
    | DYNAMIC IDENTIFIER (ASSIGN testlist_star_expr)?
    | TYPE IDENTIFIER type_params? ASSIGN type_annotation
    ;

// Type Annotations (Unions, Intersections, Nullables, Tuples, Generics)
type_annotation
    : union_type (WHERE where_constraint (COMMA where_constraint)*)?
    ;

union_type
    : intersection_type (PIPE intersection_type)*
    ;

intersection_type
    : primary_type (AMPERSAND primary_type)*
    ;

primary_type
    : base_type (LBRACK type_arg_list RBRACK | LT type_arg_list GT)? QUESTION?
    | LPAREN type_annotation (COMMA type_annotation)* RPAREN QUESTION?
    | fn_type QUESTION?
    ;

// Conditional & Loop Statements
if_stmt
    : IF expr suite (ELIF expr suite)* (ELSE suite)?
    ;

while_stmt
    : (IDENTIFIER COLON)? BOUNDED? WHILE expr suite (ELSE suite)?
    ;

for_stmt
    : (IDENTIFIER COLON)? ASYNC? PARALLEL? BOUNDED? FOR exprlist IN testlist suite (ELSE suite)?
    ;

// Pipelines, Ternary, Null Coalescing & Cascades
pipe_expr
    : conditional_expr ((PIPE_FORWARD | PIPE_BACKWARD) conditional_expr)*
    ;

conditional_expr
    : coalesce_expr (IF or_test ELSE conditional_expr)?
    ;

coalesce_expr
    : or_test (COALESCE coalesce_expr)?
    ;

cascade_arm
    : (RANGE | QUESTION_DOT_DOT) IDENTIFIER trailer* (ASSIGN expr)? (ARROW IDENTIFIER)?
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 `PilotValue` & Sized Primitive Coercion
All runtime values are represented by the immutable `readonly struct PilotValue` ([`PilotValue.cs`](../../Runtime/Values/PilotValue.cs)):
- **Zero-Allocation Unboxed Scalars**: `Int` (`long`), `Float` (`double`), `Bool` (`bool`), `None`, `Undefined`, and sized primitives (`I8`–`I64`, `U8`–`U64`, `F8`–`F64`) store their payload directly in the `8`-byte `_scalarBits` field with zero heap allocation.
- **Declaration Type Coercion**: When a variable declaration includes a type annotation (`let x: u8 = 300` or `let flag: f32 = 3.14159`), `PilotInterpreterVisitor.CoerceToDeclaredType` immediately converts and clamps/truncates the value to the target `PilotValueKind` (`U8`, `F32`, etc.).
- **Truthiness (`IsTruthy`)**:
  - `false`, `null`/`none`, `undefined`, `0`, `0.0`, `""`, empty `list` (`[]`), empty `map` (`{}`), and empty `set` evaluate to `false`.
  - All non-zero numbers, non-empty strings, non-empty collections, functions, classes, and objects evaluate to `true`.

### 4.2 Collections (`list`, `tuple`, `map`, `set`, `range`)
- **Lists (`[a, b, ..rest]`)**: Backed by `List<PilotValue>`. Supports negative indexing (`items[-1]` accesses `items[items.Count - 1]`), slice syntax (`items[1:4:1]`), and spread unpacking (`..rest` or `*rest`).
- **Tuples (`(a, b)`)**: Fixed-order heterogeneous sequences. Can be unpacked directly (`let a, b = (1, 2)`).
- **Maps (`{"k": v, **other}`)**: Backed by `Dictionary<PilotValue, PilotValue>`. Supports custom value-equality keys when objects implement `IEquatable` (via `PilotValue.Equals` and `PilotValue.GetHashCode`).
- **Sets (`{1, 2, 3}`)**: Backed by `HashSet<PilotValue>`. Non-empty braces without colons construct a `set`; `{}` constructs an empty `map`.
- **Ranges (`start..end`)**: Evaluated by `VisitComparison` when the `..` (`RANGE`) operator appears between two integers, producing an inclusive `PilotRange(start, end, step: 1)` (`0..3` yields `0, 1, 2, 3`). The built-in `range(start, stop, step)` produces a half-open range (`[start, stop)`).

### 4.3 Cascades (`..` / `?..`) & Pipelines (`|>` / `<|`)
- **Cascades**: In `primary cascade_arm+`, the receiver expression is evaluated once. Each `..member(...)` or `..prop = val` executes against the original receiver rather than the return value of the previous call. Using `?..` skips the arm if the receiver is `null`/`none`. Appending `-> alias` binds the receiver to `alias` in the current scope, and the entire cascade expression evaluates to the receiver.
- **Pipelines**: `x |> f` invokes `f(x)`. If the right-hand side is a call expression `x |> f(a, b)`, `x` is prepended as the first argument: `f(x, a, b)`. Backward pipe `f <| x` invokes `f(x)`.

---

## 5. Available Functions, Methods & Properties

### Global Conversion & Inspection Functions
| Function | Signature | Description |
| :--- | :--- | :--- |
| `type_of` | `type_of(val) -> str` | Returns the runtime type name (`"int"`, `"u16"`, `"str"`, `"list"`, `"map"`, `"set"`, or class name). |
| `len` | `len(val) -> int` | Returns character length of `str` or element count of `list`, `tuple`, `map`, `set`, `range`, or `IList`. |
| `str` / `int` / `float` / `bool` | `T(val) -> T` | Converts `val` to the target primitive type (evaluating registered `TypeAdapter` rules first). |
| `range` | `range([start,] stop[, step]) -> range` | Creates a lazy half-open integer sequence `[start, stop)`. |

### Collection Methods Summary
- **`str`**: `.upper()`, `.lower()`, `.trim()`, `.split(sep?)`, `.replace(old, new)`, `.contains(sub)`, `.starts_with(p)`, `.ends_with(s)`, `.chars()`, `.length`
- **`list`**: `.append(x)` / `.push(x)`, `.pop()`, `.clear()`, `.contains(x)`, `.index_of(x)`, `.join(sep?)`, `.reverse()`, `.sort()`, `.map(fn)`, `.filter(fn)`, `.reduce(fn[, init])`, `.for_each(fn)`, `.count` / `.length`
- **`map`**: `.keys()`, `.values()`, `.contains_key(k)` / `.has_key(k)`, `.get(k[, default])`, `.remove(k)`, `.clear()`, `.count` / `.length`
- **`set`**: `.add(x)`, `.remove(x)`, `.contains(x)`, `.union(other)`, `.intersect(other)`, `.difference(other)`, `.clear()`, `.count` / `.length`

---

## 6. Complete Runnable Examples

See [`examples/showcase/01_core_syntax_and_types.pilot`](../../examples/showcase/01_core_syntax_and_types.pilot) for the full executable showcase.

```pilot
// 1. Gradual typing, sized primitives, and union/nullable types
let inferred = 42
let byte_mask: u8 = 255
let temp: f32 = 98.6
let flexible: int | str = "ready"
let maybe_port: int? = null
let active_port = maybe_port ?? 8080

// 2. Collections with spread unpacking
let base_items = [2, 3]
let full_list = [1, ..base_items, 4]
let defaults = {"host": "localhost", "port": active_port}
let config = {**defaults, "tls": true}

// 3. Object cascade (..) with target capture (->) and safe navigation (?.)
let packet = []
    ..append("SYN")
    ..append("ACK") -> captured_packet

println(f"Captured: {captured_packet?.join('-')}") // "Captured: SYN-ACK"

// 4. Functional pipeline (|>)
def double(x: int) -> int { return x * 2 }
def add_offset(x: int, offset: int) -> int { return x + offset }

let result = 10 |> double |> add_offset(5)
assert(result == 25)

// 5. Labeled loops with inclusive range (0..3)
outer: for r in 0..3 {
    for c in 0..3 {
        if r == 1 and c == 2:
            break outer
    }
}
```

---

[🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Guard, Defer & Exceptions →](./02-guard-defer-exceptions.md)
