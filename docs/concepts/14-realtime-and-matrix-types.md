# Concept 14: Hard Real-Time Systems, Dimensional Units & Native Matrix Types

[← Prev: Metaprogramming & Lisp](./13-metaprogramming-and-lisp.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Packaging, Venv & Tooling →](./15-packaging-venv-and-tooling.md)

---

## 1. Concept Overview & Design Rationale

Pilot bridges high-level application logic with embedded, robotics, aerospace, and scientific computing through three systems-level capabilities:

1. **First-Class Dimensional Literals & Custom Units (`50ms`, `100Hz`, `64KB`, `unit`)**: Duration (`ns`, `us`, `ms`, `s`, `min`, `h`), frequency (`Hz`, `kHz`, `MHz`, `GHz`), and memory-size (`B`, `KB`, `MB`, `GB`, `TB`) literals are native lexical tokens normalized at parse/evaluation time into deterministic base units, eliminating unit-conversion bugs.
2. **Hard & Soft Real-Time Scheduling (`deadline`, `within`, `periodic`, `bounded`, `noalloc`)**: Express temporal budgets (`deadline 5ms { ... } on_missed { ... }`), soft timeouts (`within 100ms { ... } fallback { ... }`), periodic control loops (`periodic 10ms priority 99 { ... }`), WCET-bounded loops (`bounded(1000) while ...`), and deterministic allocation scopes (`noalloc`, `stackalloc`, `region`).
3. **Hardware-Sized Primitives & Native 2D Matrices (`i8`–`i64`, `u8`–`u64`, `f8`–`f64`, `matrix`)**: Exact bit-width numeric types (`u16`, `f32`, etc.) and contiguous 2D `PilotMatrix` arrays (`matrix3x3<f32>`, `eye(3)`, `zeros(r, c)`, `ones(r, c)`) with native matrix arithmetic (`+`, `-`, matrix-matrix `*`, scalar `*`, `[r, c]` indexing).

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `DURATION_LITERAL` | `<num>(ns\|us\|ms\|s\|min\|h)` | Native time-duration literal (normalizes to milliseconds `float`) |
| `FREQUENCY_LITERAL` | `<num>(Hz\|kHz\|MHz\|GHz)` | Native frequency literal (normalizes to Hertz `float`) |
| `MEMORY_LITERAL` | `<num>(B\|KB\|MB\|GB\|TB)` | Native binary memory literal (normalizes to bytes `int`) |
| `UNIT` | `unit` | Declares a custom dimensional unit (`unit N = kg * m / s**2`) |
| `DEADLINE`, `JITTER`, `ON_MISSED` | `deadline`, `jitter`, `on_missed` | Hard real-time deadline block and overrun recovery handler |
| `WITHIN`, `FALLBACK`, `TIMEOUT` | `within`, `fallback`, `timeout` | Soft real-time timeout block and fallback handler |
| `PERIODIC`, `PRIORITY` | `periodic`, `priority` | Deterministic periodic task loop |
| `BOUNDED` | `bounded` | Enforces a maximum iteration bound (`bounded(N) while ...`) |
| `NOALLOC`, `STACKALLOC`, `REGION` | `noalloc`, `stackalloc`, `region` | Deterministic zero-GC / arena memory management blocks |
| `TYPE_INT`, `TYPE_UINT`, `TYPE_FLOAT` | `i8`–`i64`, `u8`–`u64`, `f8`–`f64` | Exact bit-width integer and floating-point types |
| `TYPE_MATRIX`, `TYPE_VECTOR`, `TYPE_TENSOR` | `matrix`, `matrix2x2`–`matrix4x4`, `vector`, `vec2`–`vec4`, `tensor` | Linear algebra and SIMD matrix/vector type tokens |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// 1. Hard & Soft Real-Time Blocks
realtime_block
    : DEADLINE expr (JITTER expr)? suite (ON_MISSED suite)?
    | WITHIN expr suite ((FALLBACK | ON_MISSED | TIMEOUT) suite)?
    | PERIODIC expr (PRIORITY expr)? (JITTER expr)? suite
    | NOALLOC suite
    | REGION IDENTIFIER? (LPAREN expr RPAREN)? suite
    ;

// 2. Bounded Loops (WCET Safety)
while_stmt
    : (IDENTIFIER COLON)? BOUNDED (LPAREN expr RPAREN)? WHILE expr suite (ELSE suite)?
    | ...
    ;

for_stmt
    : (IDENTIFIER COLON)? ASYNC? PARALLEL? BOUNDED (LPAREN expr RPAREN)? FOR exprlist IN testlist suite (ELSE suite)?
    | ...
    ;

// 3. Custom Dimensional Unit Declarations & Annotations
unit_decl
    : UNIT IDENTIFIER (ASSIGN unit_expr)?
    ;

primary_type
    : base_type (LBRACK type_arg_list RBRACK | LT type_arg_list GT)? (LT unit_expr GT)? QUESTION?
    | ...
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 Dimensional Literal Normalization (`ParseDurationMs`, `ParseFrequencyHz`, `ParseMemoryBytes`)
In [`PilotInterpreterVisitor.cs`](../../Runtime/Execution/PilotInterpreterVisitor.cs):
- **`DURATION_LITERAL`**: Normalized to **milliseconds (`float`)**:
  - `1000ns` → `0.001`, `500us` → `0.5`, `25ms` → `25.0`, `2s` → `2000.0`, `1min` → `60000.0`, `1h` → `3600000.0`.
- **`FREQUENCY_LITERAL`**: Normalized to **Hertz (`float`)**:
  - `60Hz` → `60.0`, `2.4kHz` → `2400.0`, `100MHz` → `100000000.0`, `1GHz` → `1000000000.0`.
- **`MEMORY_LITERAL`**: Normalized to **exact bytes (`int` / `long`)** using binary powers of `1024`:
  - `512B` → `512`, `64KB` → `65536`, `16MB` → `16777216`, `2GB` → `2147483648`, `1TB` → `1099511627776`.

### 4.2 Real-Time Block Execution (`VisitRealtime_block`) & Bounded Loops
- **`deadline <ms> { ... } on_missed { ... }`**: Measures high-resolution elapsed time around the body using `Stopwatch.GetTimestamp()`. If elapsed milliseconds exceed the deadline budget and an `on_missed` suite is present, the `on_missed` recovery block is immediately executed.
- **`within <ms> { ... } fallback { ... }`**: Executes the primary block; if elapsed time exceeds the budget or a `PilotTimeoutException` is thrown, executes the `fallback` / `on_missed` / `timeout` block.
- **`bounded(max_iters) while` / `for`**: Maintains an iteration counter on the loop. If iterations exceed `max_iters` (default `10,000` when `(N)` is omitted), raises a catchable `PilotTimeoutException` (`TimeoutError`: `"Bounded loop exceeded maximum iteration count"`).

### 4.3 Sized Bit-Width Primitives & `PilotMatrix` Operations
- **Sized Primitives (`PilotValue.cs`)**:
  - `i8` (`sbyte`), `i16` (`short`), `i32` (`int`), `i64` (`long`)
  - `u8` (`byte`), `u16` (`ushort`), `u32` (`uint`), `u64` (`ulong`)
  - `f8` (8-bit quantized float rounded to 3 digits), `f16` (`System.Half`), `f32` (`float`), `f64` (`double`)
  - Arithmetic between two identically-sized primitives preserves the exact `PilotValueKind` (`u8 + u8 -> u8`, `f32 * f32 -> f32`) via `WrapArithmeticResult`.
- **Contiguous 2D Matrices (`PilotMatrix.cs`)**:
  - Backed by a flat row-major `double[] Data` array of size `Rows * Cols` and an `ElementType` (`"f64"`, `"f32"`, `"i32"`, etc.).
  - Supports 2D indexing (`m[r, c]` and `m[r, c] = val`), 1D row slicing (`m[r]` returns row `r` as a `list`), matrix addition (`A + B`), matrix subtraction (`A - B`), matrix dot-product multiplication (`A * B` where `A.Cols == B.Rows`), and scalar multiplication (`A * k` or `k * A`).

---

## 5. Available Functions, Methods & Properties

### Matrix Construction Built-In Functions
| Function | Signature | Description |
| :--- | :--- | :--- |
| `matrix` | `matrix(rows_list[, element_type])` or `matrix(rows, cols[, fill, element_type])` | Constructs a 2D `PilotMatrix` from nested lists or dimensions. |
| `eye` | `eye(n: int[, element_type: str = "f64"]) -> matrix` | Creates an `n × n` identity matrix with `1.0` on the diagonal. |
| `zeros` | `zeros(rows: int[, cols: int, element_type: str]) -> matrix` | Creates a zero-filled `rows × cols` matrix. |
| `ones` | `ones(rows: int[, cols: int, element_type: str]) -> matrix` | Creates a one-filled `rows × cols` matrix. |

### `PilotMatrix` Properties & Operators
| Member / Operator | Type / Signature | Description |
| :--- | :--- | :--- |
| `.rows` / `.cols` | `int` | Number of rows and columns in the matrix. |
| `.shape` | `list[int]` | `[rows, cols]` two-element list. |
| `.element_type` | `str` | Element storage type (`"f64"`, `"f32"`, `"i32"`, etc.). |
| `m[r, c]` / `m[r]` | `float \| int` / `list` | Reads or writes element `(r, c)` or returns row `r` as a `list`. |
| `A + B`, `A - B`, `A * B`, `A * k` | `matrix` | Element-wise addition/subtraction, matrix multiplication (`O(N³)` dot product), and scalar scaling. |

---

## 6. Complete Runnable Examples

See [`examples/showcase/14_realtime_and_matrix_types.pilot`](../../examples/showcase/14_realtime_and_matrix_types.pilot) and [`tests/13_builtin_types_matrix.pilot`](../../tests/13_builtin_types_matrix.pilot) for the full executable showcases.

```pilot
// 1. Native Dimensional Literals (Duration, Frequency, Memory)
let frame_budget = 16ms
let one_sec = 1s
let control_rate = 2.5kHz
let arena_size = 64KB

assert(frame_budget == 16.0 and one_sec == 1000.0)
assert(control_rate == 2500.0)
assert(arena_size == 65536)

// 2. Sized Hardware Primitives (i8..i64, u8..u64, f8..f64)
let reg_a: u16 = 1000
let reg_b: u16 = 2000
let reg_sum = reg_a + reg_b
assert(type_of(reg_sum) == "u16" and reg_sum == 3000)

// 3. Linear Algebra with Native 2D Matrices
let ident: matrix2x2<f32> = eye(2, "f32")
let transform: matrix2x2<f32> = matrix([[2.0, 3.0], [4.0, 5.0]], "f32")

let scaled = transform * 2
assert(scaled[0, 0] == 4.0 and scaled[1, 1] == 10.0)

let product = ident * transform
assert(product.rows == 2 and product.cols == 2)
assert(product[0, 1] == 3.0 and product[1, 0] == 4.0)

// 4. Hard Real-Time Deadline & Bounded WCET Loops
let deadline_ok = false
deadline 500ms {
    noalloc {
        deadline_ok = true
    }
} on_missed {
    deadline_ok = false
}
assert(deadline_ok == true)
```

---

[← Prev: Metaprogramming & Lisp](./13-metaprogramming-and-lisp.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Packaging, Venv & Tooling →](./15-packaging-venv-and-tooling.md)
