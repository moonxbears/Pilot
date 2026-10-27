# Concept 13: Metaprogramming, Homoiconic Lisp & Universal Root Object

[← Prev: Polyglot Interoperability](./12-polyglot-interoperability.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Real-Time & Matrix Types →](./14-realtime-and-matrix-types.md)

---

## 1. Concept Overview & Design Rationale

Pilot embraces Lisp's **"Code is Data, Data is Code"** philosophy while preserving modern curly-brace and indentation ergonomics:

1. **Homoiconic S-Expressions & Lisp Primitives**: Classic list-processing primitives (`cons`, `car`, `cdr`, `atom`, `symbol`, `gensym`, `eval`) are built directly into the global runtime alongside `:symbol` literals.
2. **AST Quoting, Quasiquoting & Splicing (`quote`, `` ` ``, `$`, `$@`)**: Capture arbitrary Pilot expressions or blocks as inspectable AST data (`quote(a + b)`), construct templated code fragments with `quasiquote` (or backtick `` `(...) ``), and interpolate runtime values back into templates via `unquote` (`$expr`) and `unquote_splicing` (`$@list`).
3. **Compile-Time Macros (`macro` / `defmacro`) & Embedded DSLs (`dsl`)**: Define hygienic or structural macros invoked via `name!(args)` or standard call syntax, and embed custom domain-specific sub-languages (`dsl router { ... }`).
4. **Universal Root Object Protocol**: Every value in Pilot—from primitive `int` and `bool` to `list`, `map`, and custom `class` instances—inherits a uniform 6-method reflection and serialization protocol (`to_string`, `type_of`, `hash`, `clone`, `equals`, `to_json`).

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `MACRO`, `DEFMACRO` | `macro`, `defmacro` | Declares a compile-time/AST macro or generative class macro |
| `QUOTE`, `QUASIQUOTE` | `quote`, `quasiquote` | Captures an expression or suite as an unevaluated AST / template |
| `BACKTICK` | `` ` `` | Shorthand quasiquote prefix (`` `(a + $b) ``) |
| `UNQUOTE`, `SPLICE`, `UNQUOTE_SPLICING` | `unquote`, `splice`, `unquote_splicing` | Evaluates and splices runtime values into a quasiquoted template |
| `DOLLAR`, `DOLLAR_AT` | `$`, `$@` | Shorthand unquote (`$x`) and list-splicing (`$@items`) prefixes |
| `SYMBOL_LITERAL` | `:ident` | Self-evaluating interned symbol literal (`:ok`, `:plus`) |
| `NAMEOF` | `nameof` | Compile-time identifier/expression name extraction (`nameof(x)`) |
| `DSL`, `DOMAIN`, `SYNTAX`, `OPERATOR` | `dsl`, `domain`, `syntax`, `operator` | Embedded DSL block and User-Defined Syntax Extension (UDSA) keywords |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// 1. Macro Declarations & Macro Invocation (name!(args))
macro_def
    : (MACRO | DEFMACRO) IDENTIFIER LPAREN param_list? RPAREN suite
    ;

atom
    : ...
    | IDENTIFIER BANG LPAREN arglist? RPAREN
    | IDENTIFIER BANG LBRACK arglist? RBRACK
    | IDENTIFIER BANG LBRACE stmt* RBRACE
    | quote_expr
    | splice_expr
    | nameof_expr
    ;

// 2. Quote, Quasiquote & Unquote/Splice Expressions
quote_expr
    : (QUOTE | QUASIQUOTE) (LPAREN expr RPAREN | suite)
    | BACKTICK (LPAREN expr RPAREN | primary)
    ;

splice_expr
    : (UNQUOTE | UNQUOTE_SPLICING | SPLICE) LPAREN expr RPAREN
    | DOLLAR primary
    | DOLLAR_AT primary
    ;

nameof_expr
    : NAMEOF LPAREN expr RPAREN
    ;

// 3. Embedded DSL Block
dsl_block
    : (DSL | DOMAIN) IDENTIFIER LBRACE dsl_token* RBRACE
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 S-Expression Lists & Dynamic `eval(code)`
In [`PilotBuiltinRegistry.cs`](../../Runtime/Builtins/PilotBuiltinRegistry.cs) and [`PilotInterpreterVisitor.cs`](../../Runtime/Execution/PilotInterpreterVisitor.cs):
- **Cells & Lists**: `cons(head, tail)` prepends `head` to a `list` `tail` (or creates a 2-element pair `[head, tail]`). `car(list)` returns `list[0]` (or `none` if empty), and `cdr(list)` returns `list[1..]` as a new `list`.
- **Atoms & Symbols**: `atom(x)` returns `true` for any scalar (`int`, `float`, `bool`, `str`, `symbol`, `none`) or empty list `[]`, and `false` for non-empty lists. `:name` and `symbol("name")` create `PilotValue` instances with `Kind == PilotValueKind.Symbol`. `gensym(prefix?)` uses an atomic counter (`Interlocked.Increment`) to produce collision-free symbols (`:G__1`, `:G__2`).
- **Live `eval(code)`**: Invoking `eval(str_or_quote)` parses and executes the Pilot expression or statement block inside the caller's live `PilotExecutionContext` via `Context.Engine.EvaluateExpression(src, Context)`.

### 4.2 Quoting, Quasiquoting & Unquote Substitution (`SubstituteUnquotes`)
In `VisitQuote_expr` (`PilotInterpreterVisitor.cs`):
- **`quote(expr)`**: Returns the raw source representation of `expr` without evaluating it.
- **`quasiquote(expr)` / `` `(expr) ``**: Walks the source text via `SubstituteUnquotes`:
  - Every `$var`, `$(expr)`, `unquote(expr)`, or `splice(expr)` is evaluated in the current scope and formatted into the code template.
  - Every `$@list` or `unquote_splicing(list)` evaluates the list and splices its comma-separated elements directly into the template.

### 4.3 Universal Root Object Dispatch (`TryEvaluateUniversalRootMethod`)
Before checking type-specific methods on **any** `PilotValue`, `EvaluatePostfix` checks `TryEvaluateUniversalRootMethod`:
- Every value (`42`, `"hello"`, `[1, 2]`, `{"a": 1}`, or custom `Class` instance) responds to `.to_string()`, `.type_of()`, `.hash()`, `.clone()`, `.equals(other)`, and `.to_json()`.
- On custom `PilotObject` instances, if the class defines its own `to_string`, `equals`, or `hash` method (or delegates `IEquatable`), the class's implementation is invoked; otherwise, Pilot falls back to the structural Universal Root Object implementation (`CloneValue` deep-copies fields/collections; `SerializeToJson` recursively emits RFC 8259 JSON).

---

## 5. Available Functions, Methods & Properties

### Lisp & Metaprogramming Built-In Functions
| Function | Signature | Description |
| :--- | :--- | :--- |
| `cons` | `cons(head, tail) -> list` | Prepends `head` onto list `tail` (or constructs `[head, tail]`). |
| `car` | `car(list) -> any` | Returns the first element (`head`) of `list`, or `none` if empty. |
| `cdr` | `cdr(list) -> list` | Returns a new `list` containing all elements after the first. |
| `atom` | `atom(val) -> bool` | Returns `true` if `val` is not a non-empty `list`. |
| `symbol` | `symbol(name: str) -> symbol` | Interns `name` as a `:symbol` value. |
| `gensym` | `gensym([prefix: str]) -> symbol` | Generates a unique symbol (`:G__1`, `:tmp__2`) for hygienic macros. |
| `eval` | `eval(code: str \| symbol) -> any` | Dynamically parses and evaluates `code` in the active lexical scope. |
| `nameof` | `nameof(expr) -> str` | Returns the source identifier or expression name as a `str`. |

### Universal Root Object Methods (Available on All Values)
| Method | Aliases | Signature | Description |
| :--- | :--- | :--- | :--- |
| `.to_string()` | `.__str__()` | `val.to_string() -> str` | Returns the string representation of `val`. |
| `.type_of()` | `.get_type()` | `val.type_of() -> str` | Returns the runtime type name of `val`. |
| `.hash()` | `.get_hash_code()` | `val.hash() -> int` | Returns the deterministic 32-bit integer hash code of `val`. |
| `.clone()` | — | `val.clone() -> any` | Deep-clones lists, maps, sets, and `PilotObject` field graphs. |
| `.equals(other)` | `.__eq__(other)` | `val.equals(other) -> bool` | Value and structural equality check. |
| `.to_json()` | — | `val.to_json() -> str` | Serializes any scalar, collection, or `PilotObject` to a JSON string. |

---

## 6. Complete Runnable Examples

See [`examples/showcase/13_metaprogramming_lisp_and_root_object.pilot`](../../examples/showcase/13_metaprogramming_lisp_and_root_object.pilot) for the full executable showcase.

```pilot
// 1. Homoiconic Lisp primitives: cons, car, cdr, atom, symbol, gensym
let s_expr = cons(:add, [10, 20, 30])
assert(car(s_expr) == :add)
assert(cdr(s_expr) == [10, 20, 30])
assert(atom(:add) == true and atom(s_expr) == false)

let uniq1 = gensym("var_")
let uniq2 = gensym("var_")
assert(uniq1 != uniq2)

// 2. AST Quoting, Quasiquoting (`) with Unquote ($) & Splicing ($@), and eval()
let factor = 6
let extra_args = [2, 3]
let raw_ast = quote(10 + 20)
assert(raw_ast == "10 + 20")

let templated_code = `(7 * $factor)
assert(eval(templated_code) == 42)

let list_template = `[1, $@extra_args, 4]
assert(eval(list_template) == [1, 2, 3, 4])

// 3. Compile-time / AST Macro Definition & Invocation
macro unless(cond, fallback_val) {
    if not cond {
        return fallback_val
    }
    return "condition_was_true"
}
assert(unless!(false, "triggered_fallback") == "triggered_fallback")

// 4. Universal Root Object methods on primitives, collections, and objects
assert(42.to_string() == "42" and 42.type_of() == "int")

let original = {"items": [1, 2], "status": "ok"}
let cloned = original.clone()
cloned["items"].append(3)
assert(len(original["items"]) == 2 and len(cloned["items"]) == 3)
assert(original.to_json() == "{\"items\":[1,2],\"status\":\"ok\"}")
```

---

[← Prev: Polyglot Interoperability](./12-polyglot-interoperability.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Real-Time & Matrix Types →](./14-realtime-and-matrix-types.md)
