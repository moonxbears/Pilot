# Concept 06: Structural Projections, Scoped Lexical Extensions & Generative Classes

[← Prev: Algebraic Classes & Typestates](./05-algebraic-classes-and-typestates.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Contextual Parameters & DI →](./07-contextual-parameters-di.md)

---

## 1. Concept Overview & Design Rationale

Pilot provides three high-leverage structural and metaprogramming tools for shaping objects and types without polluting global namespaces or hand-writing repetitive DTOs:

1. **Structural Projection (`obj -> { field1, field2 }`)**: Extracts a subset of fields, computed properties, or zero-argument methods from any object or map into a lightweight anonymous DTO (`map`) in a single expression—ideal for API serialization, redacting sensitive fields (`password_hash`), or shaping query outputs.
2. **Scoped Lexical Extensions (`extension Name on Type`)**: Unlike C# or Ruby where extension methods or monkey-patches leak globally across an entire project, Pilot extensions are **named and lexically scoped**. They exist only inside an explicit `with extension Name { ... }` block and vanish automatically when the block exits.
3. **Compile-Time Generative Classes (`macro` inside `class`)**: Classes can define or inherit `macro(ast)` hooks that receive a mutable `PilotClassNode` AST representation when the class (or any subclass) is defined. Macros can inspect declared fields and dynamically synthesize getters, setters, equality checks, audit fields, or serialization methods at class-creation time.

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `ARROW` | `->` | Structural projection operator (`user -> { id, name }`) |
| `LBRACE`, `RBRACE`, `LBRACK`, `RBRACK` | `{`, `}`, `[`, `]` | Identifier-list projection (`{ a, b }`) or string-list projection (`["a", "b"]`) |
| `EXTENSION`, `ON` | `extension`, `on` | Declares a named lexical extension for a target type |
| `WITH` | `with` | Activates one or more lexical extensions (`with extension Name`) |
| `MACRO`, `DEFMACRO` | `macro`, `defmacro` | Declares a compile-time generative class macro inside a `class` body |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// 1. Structural Projection Expression
projection_expr
    : unary_expr (ARROW LBRACE IDENTIFIER (COMMA IDENTIFIER)* RBRACE)?
    ;
// Also supported via binary '->' with a list of field-name strings:
// obj -> ["id", "email"]

// 2. Scoped Lexical Extension Declaration & Activation
extension_decl
    : EXTENSION IDENTIFIER? ON type_annotation suite
    ;

with_item
    : EXTENSION IDENTIFIER      // Activated inside: with extension MathExt { ... }
    | expr (AS expr)?
    ;

// 3. Generative Class Macro Inside a Class Body
class_member
    : ...
    | macro_def
    ;

macro_def
    : (MACRO | DEFMACRO) IDENTIFIER LPAREN param_list? RPAREN suite
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 Structural Projections (`VisitProjection_expr` & `EvaluateProjection`)
In [`PilotInterpreterVisitor.cs`](../../Runtime/Execution/PilotInterpreterVisitor.cs):
- When `target -> { f1, f2 }` (or `target -> ["f1", "f2"]`) is evaluated, `EvaluateProjection` iterates over the requested field names and constructs a new `Dictionary<PilotValue, PilotValue>` (`map`):
  - **On a `PilotObject`**: Evaluates `EvaluateObjectMember(obj, field)`. If the member is a field or `get` property, its value is stored under `"f1"`. If the member is a zero-argument method, it is automatically invoked (`CallFunction(fn, [])`) and its return value is projected into the resulting `map`.
  - **On a `map`**: Copies matching string keys into the new projected `map`, or sets `none` if absent.

### 4.2 Scoped Lexical Extensions (`PilotLexicalExtension`)
In [`PilotLexicalExtension.cs`](../../Runtime/Extensions/PilotLexicalExtension.cs) and `PilotExecutionContext`:
1. **Declaration (`VisitExtension_decl`)**: Creates a `PilotLexicalExtension(name, targetType)` containing all `def` methods inside the block, where each method binds `self` to the receiver instance. The extension is registered in `Context.DefineExtension(ext)`.
2. **Scoped Activation (`with extension Name`)**: `VisitWith_stmt` calls `Context.PushExtension("Name")` on entry and `Context.PopExtension("Name")` in the `finally` block.
3. **Member Dispatch (`TryResolveExtensionMethod`)**: Whenever a method call `val.method(args)` is evaluated on **any** `PilotValue` (`str`, `int`, `list`, or custom `Class`), `PilotInterpreterVisitor` searches `Context.ActiveExtensions` in LIFO order. If an active extension targets `val.TypeName` (or `"any"` / `"dynamic"` / `"object"`), the extension method is bound with `self = val` and invoked. Outside the `with extension` block, calling `val.method()` fails with a normal `TypeError`.

### 4.3 Compile-Time Generative Classes (`PilotClassNode`)
In [`PilotClassNode.cs`](../../Runtime/Generative/PilotClassNode.cs) and `VisitClass_def`:
1. When a `class` is evaluated, any `macro name(ast)` blocks declared in the class body are registered in `pilotClass.ClassMacros`.
2. Next, `VisitClass_def` collects all `ClassMacros` from every base class (top-down along the inheritance chain) plus the class's own `ClassMacros`.
3. If any class macros exist, `pilotClass` is wrapped in a `PilotClassNode` (`Kind == PilotValueKind.ClassNode`) and passed as the `ast` argument to each macro function (`CallFunction(macroFn, [PilotValue.FromClassNode(classNode)])`).
4. Inside the macro body, calling `ast.add_field(name, default)` or `ast.add_method(name, fn)` mutates the underlying `PilotClass` directly before any instances are ever constructed!

---

## 5. Available Functions, Methods & Properties

### `PilotClassNode` (`ast`) Members Inside Class Macros
| Member | Kind | Signature / Type | Description |
| :--- | :--- | :--- | :--- |
| `ast.name` | Property | `str` | Name of the class currently being generated (e.g., `"Customer"`). |
| `ast.fields` | Property | `list[str]` | List of all field names currently declared on the class. |
| `ast.methods` | Property | `list[str]` | List of all method names currently declared on the class. |
| `ast.add_field` | Method | `ast.add_field(name: str[, default_val]) -> none` | Injects a new field into the class with an optional default value. |
| `ast.add_method` | Method | `ast.add_method(name: str, fn: callable) -> none` | Injects a new method into the class. In `fn`, the first parameter receives the instance (`self`). |

---

## 6. Complete Runnable Examples

See [`examples/showcase/06_projections_extensions_and_generative.pilot`](../../examples/showcase/06_projections_extensions_and_generative.pilot) for the full executable showcase.

```pilot
// 1. Structural Projection (obj -> { f1, f2 })
class Account {
    let id: int = 7
    let username: str = "ada"
    let password_hash: str = "sha256:secret"
    def role_badge() -> str { return "ADMIN" }
}

let acc = Account()
// Projects fields AND zero-arg methods into a clean DTO map, omitting password_hash
let public_dto = acc -> { id, username, role_badge }
assert(public_dto == {"id": 7, "username": "ada", "role_badge": "ADMIN"})

// 2. Scoped Lexical Extensions (extension Name on Type)
extension CurrencyFormatting on float {
    def to_usd() -> str {
        return f"${self}"
    }
}

let amount = 49.95
with extension CurrencyFormatting {
    assert(amount.to_usd() == "$49.95")
}
// Outside the 'with extension' block, float has no .to_usd() method!

// 3. Compile-Time Generative Classes (Inherited Class Macros)
class AutoGetters {
    macro synthesize_accessors(ast) {
        // Automatically inject an 'entity_type' field and 'get_<field>' methods
        ast.add_field("entity_type", ast.name)
        for field_name in ast.fields {
            let captured = field_name
            ast.add_method(f"get_{captured}", (self) => self.get_field(captured))
        }
    }
}

// Product inherits AutoGetters, triggering 'synthesize_accessors(ast)' at definition time!
class Product : AutoGetters {
    let sku: str = "PILOT-01"
    let price: int = 199
}

let item = Product()
assert(item.entity_type == "Product")
assert(item.get_sku() == "PILOT-01")
assert(item.get_price() == 199)
```

---

[← Prev: Algebraic Classes & Typestates](./05-algebraic-classes-and-typestates.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Contextual Parameters & DI →](./07-contextual-parameters-di.md)
