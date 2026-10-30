# Concept 12: Polyglot Interoperability & Resumable Yield-Bridge Coroutines

[← Prev: Concurrency, Generators & Lazy](./11-concurrency-generators-lazy.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Metaprogramming & Lisp →](./13-metaprogramming-and-lisp.md)

---

## 1. Concept Overview & Design Rationale

Pilot is engineered as a **universal polyglot glue language**. Rather than forcing developers to rewrite battle-tested libraries or maintain brittle subprocess wrappers, Pilot embeds 11 foreign language ecosystems directly inside `.pilot` source files:

1. **Inline Polyglot Blocks (`python { ... }`, `rust { ... }`, `js { ... }`, etc.)**: Embed Python, C#, Rust, JavaScript, Java, Lua, C, C++, Assembly, PostgreSQL, or MySQL blocks directly inside Pilot functions or scripts using braces (`{ ... }`), triple-quoted strings (`"""..."""`), or markdown-style code fences (```` ```python ... ``` ````).
2. **Bidirectional Variable Sharing**: Pilot variables in the enclosing scope are automatically marshaled into the foreign language block, and variables assigned or returned inside the foreign block are synchronized back into the Pilot scope.
3. **Resumable Polyglot Coroutines (The Yield-Bridge)**: When a Pilot generator function (`def stream()`) contains `yield` statements inside an inline `python { ... }` or `js { ... }` block, Pilot suspends the coroutine across the language boundary—preserving local state on both sides as `.next()` is called!
4. **Foreign Module Imports (`import python:math`) & Bidirectional Transpilation**: Import foreign modules directly into Pilot namespaces or transpile entire source files between Pilot, Python, C#, and Rust via `--emit` and `--from`.

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `LANG_PYTHON`, `LANG_CSHARP`, `LANG_RUST` | `python`, `csharp`, `rust` | Inline polyglot block and foreign import language identifiers |
| `LANG_JS`, `LANG_JAVA`, `LANG_LUA` | `js`, `javascript`, `java`, `lua` | Scripting & JVM polyglot language identifiers |
| `LANG_C`, `LANG_CPP`, `LANG_ASM` | `c`, `cpp`, `c++`, `asm`, `assembly` | Systems & native assembly polyglot language identifiers |
| `LANG_POSTGRES`, `LANG_MYSQL` | `postgresql`, `postgres`, `sql`, `mysql` | Inline SQL polyglot block identifiers |
| `EXTERN`, `LIB` | `extern`, `lib` | Native C/Rust FFI library bindings (`extern "C" lib "m"`) |
| `RAW_FOREIGN_BLOCK` | ```` ```lang ... ``` ```` | Raw fenced polyglot block literal |
| `TRIPLE_STRING` | `"""..."""`, `'''...'''` | Multi-line string body for polyglot blocks |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// 1. Inline Polyglot Blocks (Statement or Expression)
inline_foreign_block
    : lang_tag (LBRACE stmt* RBRACE | TRIPLE_STRING | RAW_FOREIGN_BLOCK)
    ;

lang_tag
    : LANG_C | LANG_CPP | LANG_CSHARP | LANG_JAVA | LANG_JS
    | LANG_PYTHON | LANG_RUST | LANG_LUA | LANG_ASM | LANG_POSTGRES | LANG_MYSQL
    ;

// 2. Foreign Ecosystem Module Imports
import_stmt
    : IMPORT lang_tag COLON dotted_name (AS IDENTIFIER)?
    | FROM lang_tag COLON dotted_name IMPORT (STAR | import_as_names)
    | ...
    ;

// 3. Native FFI Extern Block
extern_block
    : EXTERN STRING? (LIB STRING)? LBRACE NEWLINE? extern_member* RBRACE
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 Two Execution Forms in Polyglot Blocks
In [`crates/pilot-polyglot/src/bridge.rs`](../../crates/pilot-polyglot/src/bridge.rs) and [`crates/pilot-codegen/src/c_target.rs`](../../crates/pilot-codegen/src/c_target.rs):
1. **AST-Integrated Brace Form (`python { ... }`, `js { ... }`, `rust { ... }`)**:
   - The statements inside `{ ... }` are parsed into Pilot's unified AST (`NodeKind::PolyglotBlock`) and executed in the active scope.
   - Any `yield` statement inside `python { yield x }` or `js { yield y }` interacts directly with Pilot's generator mechanics, enabling **zero-copy resumable polyglot coroutines** across language blocks.
2. **Raw / Triple-String Form (`python """..."""` or ````python ```...``` ````)**:
   - Extracted as raw source text.
   - All currently visible bindings in the active frame are snapshotted into a foreign environment map and passed to [`pilot-polyglot::bridge`](../../crates/pilot-polyglot/src/bridge.rs).
   - Any variables mutated or created in the foreign scope are written back into the active Pilot scope, and the final expression result is returned.

### 4.2 Polyglot Bridge & Foreign Module Imports (`import lang:module`)
In [`crates/pilot-polyglot/src/marshal.rs`](../../crates/pilot-polyglot/src/marshal.rs):
- Executing `import python:math as py_math` calls the polyglot bridge import mechanism, returning a map of callable function and constant bindings:
  - `python:math`: `sqrt(x)`, `sin(x)`, `cos(x)`, `pow(x, y)`, `floor(x)`, `ceil(x)`, `abs(x)`, `pi`, `e`
  - `python:json`: `dumps(val)`, `loads(str)`
  - `python:os`: `getcwd()`, `getenv(name)`
  - `js:Math` / `javascript:Math`: `max(*args)`, `min(*args)`, `abs(x)`, `round(x)`, `floor(x)`, `sqrt(x)`, `PI`
  - `js:JSON`: `stringify(val)`, `parse(str)`
  - `csharp:System.Math`: `Max(a, b)`, `Min(a, b)`, `Abs(x)`, `Sqrt(x)`, `Pow(x, y)`, `PI`
  - `rust:std`: `mem_size_of(val)`, `type_name_of(val)`
  - `lua:string`: `upper(s)`, `lower(s)`, `len(s)`, `reverse(s)`
  - `c:math` / `cpp:std`: `sqrt(x)`, `abs(x)`, `pow(x, y)`, `sin(x)`, `cos(x)`

---

## 5. Available Functions, Methods & CLI Flags

| Feature / CLI Flag | Syntax | Description |
| :--- | :--- | :--- |
| Inline Polyglot Block | `<lang> { ... }` or `<lang> """..."""` | Executes embedded foreign code with shared scope variables and return value. |
| Foreign Module Import | `import <lang>:<module> [as <alias>]` | Imports a foreign ecosystem library as a Pilot namespace map. |
| Emit Foreign Code | `pilot --emit <python\|csharp\|rust> file.pilot` | Transpiles a `.pilot` source file into idiomatic Python, C#, or Rust (`Transpilation/PilotEmitter.cs`). |
| Import Foreign File | `pilot --from <python\|csharp\|rust\|js> file.ext` | Converts foreign source code into `.pilot` syntax (`Transpilation/ForeignToPilotImporter.cs`). |

---

## 6. Complete Runnable Examples

See [`examples/showcase/12_polyglot_and_coroutines.pilot`](../../examples/showcase/12_polyglot_and_coroutines.pilot) for the full executable showcase.

```pilot
// 1. Foreign module imports across Python, JavaScript, and Rust
import python:math as py_math
import js:Math as js_math
import rust:std as rs_std

assert(py_math["sqrt"](144) == 12.0)
assert(js_math["max"](10, 50, 30) == 50.0)
assert(rs_std["mem_size_of"](42) == 8)

// 2. Inline polyglot blocks sharing Pilot scope variables
let base_val = 10
let multiplier = 4

let computed = python {
    let intermediate = base_val * multiplier
    intermediate + 2
}
assert(computed == 42)

// 3. Resumable Polyglot Yield-Bridge Coroutine
def polyglot_pipeline(seed: int) {
    python {
        let step1 = seed + 10
        yield f"py:{step1}"
    }
    js {
        let step2 = seed * 3
        yield f"js:{step2}"
    }
    rust {
        let step3 = seed + 100
        yield f"rs:{step3}"
    }
}

let stream = polyglot_pipeline(5)
assert(stream.next() == "py:15")
assert(stream.next() == "js:15")
assert(stream.next() == "rs:105")

// 4. Inline PostgreSQL block with live variable interpolation
let target_dept = "Engineering"
let sql_query = postgresql """
    SELECT id, name FROM employees WHERE department = {target_dept}
"""
assert(sql_query.contains("Engineering"))
```

---

[← Prev: Concurrency, Generators & Lazy](./11-concurrency-generators-lazy.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Metaprogramming & Lisp →](./13-metaprogramming-and-lisp.md)
