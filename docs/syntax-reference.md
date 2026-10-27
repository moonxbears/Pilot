# Pilot Syntax & Grammar Reference

[**← Lexical Reference**](lexical-reference.md) | [**Documentation Hub**](index.md) | **Syntax Reference** | [**Functions & API Reference →**](functions-and-apis.md)

---

This reference provides a complete specification of Pilot's syntax, grammar rules, statement forms, type system expressions, and operator precedence hierarchy as defined in [`antlr/Pilot.g4`](../antlr/Pilot.g4) and executed by [`Runtime/Execution/PilotInterpreterVisitor.cs`](../Runtime/Execution/PilotInterpreterVisitor.cs).

## Table of Contents

1. [Program Structure, Statements & Suites](#1-program-structure-statements--suites)
2. [Type System & Type Annotations](#2-type-system--type-annotations)
3. [Variable, Constant & Destructuring Declarations](#3-variable-constant--destructuring-declarations)
4. [Functions, Lambdas, Parameters & Contracts](#4-functions-lambdas-parameters--contracts)
5. [Classes, Typestates, Structs, Interfaces, Enums & Actors](#5-classes-typestates-structs-interfaces-enums--actors)
6. [Properties, Operator Overloading, Extensions & Type Adapters](#6-properties-operator-overloading-extensions--type-adapters)
7. [Control Flow, `guard`, `defer`, `try` & `with` Statements](#7-control-flow-guard-defer-try--with-statements)
8. [Pattern Matching (`match` & `is`)](#8-pattern-matching-match--is)
9. [Relational Database & LINQ/SQL Query Syntax](#9-relational-database--linqsql-query-syntax)
10. [Concurrency, Channels & `select` Syntax](#10-concurrency-channels--select-syntax)
11. [Real-Time Determinism & Memory Control Syntax](#11-real-time-determinism--memory-control-syntax)
12. [Polyglot Interoperability, Macros & UDSA Syntax](#12-polyglot-interoperability-macros--udsa-syntax)
13. [Expression Hierarchy & Operator Precedence Table](#13-expression-hierarchy--operator-precedence-table)

---

## 1. Program Structure, Statements & Suites

A Pilot program (`program`) is a sequence of zero or more top-level statements (`stmt* EOF`). Top-level statements execute directly without requiring a `main()` boilerplate wrapper, and `defer` statements registered at top level execute when the script finishes.

```ebnf
program       ::= stmt* EOF
stmt          ::= compound_stmt | simple_stmt
simple_stmt   ::= small_stmt (';' small_stmt)* ';'?
suite         ::= '{' stmt* '}' | simple_stmt
```

###Flexible Block Delimiters (`:` and `{ ... }`)
Pilot supports both C#/Rust-style braces (`{ ... }`) and Python-style colons (`:`) before any `suite`:
- **Braced Multi-Statement Block**: `if x > 0 { println(x); return x; }`
- **Colon + Braced Block**: `if x > 0: { println(x); }`
- **Colon + Single/Semicolon-Separated Simple Statement**: `def square(x: int): return x * x;`

Semicolons (`;`) at the end of simple statements are **optional**, though they can be used to place multiple `small_stmt` items on a single line.

---

## 2. Type System & Type Annotations

Type annotations follow a colon (`: type_spec`) on variables, fields, and parameters, or an arrow (`-> type_spec`) on function return types.

### 2.1 Grammar Rules
```ebnf
type_annotation   ::= ':' type_spec
return_type       ::= '->' type_spec
type_spec         ::= union_type
union_type        ::= intersection_type ('|' intersection_type)*
intersection_type ::= primary_type ('&' primary_type)*
primary_type      ::= 'ref'? base_type '?'? ('<' unit_expr '>')?
base_type         ::= 'symbol'
                    | 'dynamic'
                    | (dotted_name | 'none' | 'null' | 'undefined') ( '[' type_arguments ']' | '<' type_arguments '>' )?
                    | '(' type_spec (',' type_spec)* ')'
                    | '(' (type_spec (',' type_spec)*)? ')' '->' type_spec
                    | '[' type_spec (';' test)? ']'
                    | '{' type_spec ':' type_spec '}'
```

### 2.2 Supported Type Forms

| Type Category | Syntax Examples | Meaning |
| :--- | :--- | :--- |
| **Primitive Types** | `int`, `i8`, `i16`, `i32`, `i64`, `uint`, `u8`, `u16`, `u32`, `u64`, `float`, `f8`, `f16`, `f32`, `f64`, `double`, `decimal`, `complex`, `bool`, `char`, `str`, `string` | Sized or standard numeric, boolean, character, and string types. |
| **Collection & Matrix Types** | `list`, `list<int>`, `List[f64]`, `tuple<int, str>`, `map<str, int>`, `set<int>`, `frozenset<str>`, `range`, `bytes`, `bytearray`, `memoryview`, `matrix3x3<u8>`, `matrix4x4<f32>` | Built-in collection, buffer, and dimensional matrix types. Generic type arguments accept either `<T>` or `[T]`. |
| **Structural Shorthand Types** | `[int]` (list), `[u8; 64]` (fixed-length array), `(int, str)` (tuple), `{str: int}` (map), `(int, int) -> bool` (function type) | Structural type notations for lists, fixed arrays, tuples, maps, and callables. |
| **Nullable Types** | `int?`, `User?` | Appending `?` permits `null` / `none` in addition to the base type. |
| **Union & Intersection Types** | `int \| str \| undefined`, `Readable & Serializable` | `\|` forms a union type; `&` forms an intersection type. |
| **Dimensional Unit Types** | `float<m/s>`, `f64<kg * m / s**2>` | Attaches a physical unit expression `<unit_expr>` to a numeric type. |
| **Reference & Dynamic Types** | `ref int`, `dynamic`, `any`, `symbol` | By-reference storage (`ref`), dynamic type-check bypass (`dynamic` / `any`), and interned symbols (`symbol`). |
| **Generic Parameters & `where` Clauses** | `def sort[T: IComparable](items: list<T>) where T: IEquatable` | Generic parameters declared in `[...]` with optional inline `: Type` or trailing C#-style `where T: Constraint` clauses. |

---

## 3. Variable, Constant & Destructuring Declarations

### 3.1 Declaration Statements (`small_stmt`)

| Statement | Grammar Rule | Semantics |
| :--- | :--- | :--- |
| **`let` Declaration** | `declaration_modifier* 'let' binding_pattern (':' type_spec)? ('=' test)?` | Declares a variable or class field in the current scope (`isConst = false`). Defaults to `undefined` if `= test` is omitted. |
| **`var` Declaration** | `declaration_modifier* 'var' binding_pattern (':' type_spec)? ('=' test)? index_clause?` | Declares a mutable variable or an in-memory indexed relation (`var users = [...] index by (id [hash, unique])`). |
| **`const` Declaration** | `declaration_modifier* 'const' binding_pattern (':' type_spec)? '=' test` | Declares an immutable constant (`isConst = true`). Requires an initializer expression. Attempting to reassign a `const` binding fails. |
| **`ref` Declaration** | `declaration_modifier* 'ref' binding_pattern (':' type_spec)? ('=' test)?` | Declares a reference-aliased variable binding. |
| **`dynamic` Declaration** | `declaration_modifier* 'dynamic' binding_pattern ('=' test)?` | Declares a dynamically typed variable binding. |
| **Bare Type Annotation** | `test ':' type_spec ('=' test)?` | Python/TypeScript-style field or variable declaration without `let`/`var` (e.g., `name: str` inside a class body). |
| **Type Alias** | `declaration_modifier* 'type' identifier generic_params? '=' type_spec` | Defines a named alias for a complex type expression (`type StringMap[V] = {str: V}`). |
| **Global Binding** | `'global' identifier (',' identifier)*` | Binds the listed identifiers to the global scope inside a function. |

### 3.2 Destructuring Binding Patterns (`binding_pattern`)
Any `let`, `var`, `const`, `for` target, or standalone `pattern_assign_stmt` can destructure nested tuples, lists, maps, or objects:

```ebnf
binding_pattern       ::= identifier | splice_expr | tuple_pattern | list_pattern | record_pattern
tuple_pattern         ::= '(' (binding_pattern_item (',' binding_pattern_item)* ','?)? ')'
list_pattern          ::= '[' (binding_pattern_item (',' binding_pattern_item)* ','?)? ']'
record_pattern        ::= '{' (record_pattern_field (',' record_pattern_field)* ','?)? '}'
binding_pattern_item  ::= ('*' | '**' | '..')? binding_pattern (':' type_spec)? ('=' test)?
record_pattern_field  ::= (identifier | STRING) (':' binding_pattern)? ('=' test)?
                        | '**' identifier
pattern_assign_stmt   ::= (tuple_pattern | list_pattern | record_pattern) '=' test
```

**Exact Runtime Destructuring Rules (`PilotInterpreterVisitor.cs`)**:
- **List/Tuple Patterns**: Verify that the right-hand side is a `List` or `Tuple`. If no rest element (`..rest` or `*rest`) is present, the element count must match **exactly** or `PilotPatternMatchingException` (`MatchError`) is thrown. If a rest element is present (`[head, ..middle, tail] = seq`), `head` binds prefix items, `tail` binds suffix items, and `middle` binds all remaining interior items as a new `list`.
- **Record Patterns**: Verify that the right-hand side is a `Map` or `Object`. Extracts each named property (`{ "id": user_id, "meta": { "role": role = "guest" }, **rest } = payload`). If a required key is missing and has no `= default` expression, `PilotPatternMatchingException` is thrown. `**rest` collects all unvisited keys into a new `map`.

---

## 4. Functions, Lambdas, Parameters & Contracts

### 4.1 Function & Method Declarations (`func_def`)
```ebnf
func_def        ::= decorators? declaration_modifier* ('pure' | 'async')? ('def' | 'fn')
                    identifier generic_params? '(' parameter_list? ')' ('->' type_spec)?
                    where_type_clause? ':'? contract_clause* ':'? (suite | ';')?
parameter       ::= 'context'? ('each' | 'repeat')? ('params' | 'ref')? ('*' | '**')?
                    (identifier | splice_expr) (':' type_spec)? ('=' test)?
contract_clause ::= 'requires' test | 'ensures' test
```

### 4.2 Parameter Modifiers & Exact Behavior

| Modifier | Syntax | Exact Runtime Behavior |
| :--- | :--- | :--- |
| **`context`** | `context db: Database` | **Contextual Parameter (Native DI)**: If not explicitly supplied at the call site, `PilotEngine.CreateCallScope` automatically resolves the parameter name from the caller's `ContextualBindings` stack (`with context db = ...`), falling back to `= default` or throwing `PilotContextException` (`ContextError`). |
| **`each` / `repeat`** | `def calc(each x: int)` | Marks the parameter for **automatic scalar vectorization**. Passing a `list`, `tuple`, or `set` to `x` automatically unrolls the call across each element and returns a `list` of results. |
| **`params` / `*`** | `params rest: list` or `*args` | Variadic parameter that collects all remaining trailing positional arguments into a `PilotValueKind.List`. |
| **`**`** | `**kwargs` | Collects keyword arguments. |
| **`ref`** | `ref count: int` | Marks parameter as passed by reference. |
| **Type Annotation** | `x: int` | Used by **Method Group Dispatch** to route calls to matching overloads and by **Type Adapters** (`PilotTypeAdapterRegistry`) to automatically convert compatible arguments (e.g., `"42"` -> `42`, `{...}` -> `Model`). |
| **Default Value** | `timeout: int = 30` | Evaluated and bound when the argument is omitted by the caller. |

### 4.3 Automatic Method Group Promotion
When multiple functions with the **same name** are declared in the same scope (or class/state/extension), `VisitFunc_def` automatically promotes the binding from a single `PilotFunction` into a **`PilotMethodGroup`** containing all overloads.

### 4.4 Generator Detection
During `VisitFunc_def`, `ContainsYield(context.suite())` inspects the function's AST (excluding nested inner `func_def` nodes). If any `yield` statement is present, `IsGenerator` is set to `true`, and calling the function returns a stateful `PilotGenerator` coroutine instead of executing eagerly.

### 4.5 Lambda Expressions (`lambda_expr`)
Pilot supports six lambda syntax forms (C++, C#/JS, Rust, and Python styles):
1. **C++ Capture + Parameters + Fat Arrow/Braces**: `[ref x, y = 10](a: int) -> int => a + x + y` or `[&]() { ... }`
2. **Typed/Parenthesized Fat Arrow**: `(a: int, b: int) -> int => a + b` or `(a, b) => { return a + b; }`
3. **Single-Identifier Fat Arrow**: `x => x * 2` or `x => { return x * 2; }`
4. **`def` / `fn` Anonymous Function**: `fn(x: int) -> int { return x * 2; }` or `def(x) => x + 1`
5. **Python-style `lambda`**: `lambda x, y=1: x + y` or `lambda x -> x * 2`

---

## 5. Classes, Typestates, Structs, Interfaces, Enums & Actors

### 5.1 Class Definitions & Algebraic Class Math (`class_def`)
```ebnf
class_def ::= decorators? declaration_modifier* 'class' identifier generic_params? '=' test ';'?
            | decorators? declaration_modifier* 'class' identifier generic_params? ('(' class_bases? ')')? where_type_clause? ':'? suite
```
- **Standard Class**: `class Dog(Animal) { let breed = "Husky"; def __init__(self, name) { self.name = name; } }`
  - Constructors can be named `__init__`, `constructor`, or the class's own name (`Dog`).
  - Declaring a class with the same name as an existing class in scope merges its members (supporting `partial class`).
- **Algebraic Class Composition**: `class Admin = BaseUser | AdminPrivileges;`
  - Union (`ClassA | ClassB`): Combines static/default fields, methods, method groups, and typestates from both classes.
  - Subtraction (`ClassA - ClassB` or `ClassA - "member_name"`): Clones `ClassA` and removes all fields/methods/states present in `ClassB` (or the named member).
  - Intersection (`ClassA & ClassB`): Keeps only fields, methods, and method groups common to both `ClassA` and `ClassB`.

### 5.2 Typestate Blocks (`state_def`)
```ebnf
state_def ::= 'state' identifier generic_params? ':'? suite
```
Defined inside a `class` body.
- The **first** `state` block in a class automatically becomes `classDef.InitialState`.
- When an instance is in state `S`, calling a method defined in state `S` executes normally; calling a method defined exclusively in another state `T` of the same class throws `PilotTypeException`:
  `Method or member '<name>' does not exist on <Class>[<S>]. Valid state: <T>.`
- State transitions occur via `self.transition_to("NewState")` and can be inspected via `self.current_state()`.

### 5.3 Contract Delegation & Method Stealing (`steal_stmt`, `delegate_stmt`, `contract_forward_stmt`)
Inside a class body or constructor:
- **Method Stealing (`steal`)**:
  - `steal EncryptionEngine;` — Copies all non-constructor methods from `EncryptionEngine` into the current class without inheritance (existing methods on the current class are never overwritten).
  - `steal from ScientificOps [exp, log];` — Selectively steals only the specified methods.
- **Contract Forwarding / Delegation**:
  - `IList.Contracts = _items;` or `IList<PilotValue>.Contracts = _items;`
  - `contracts for IList = _items;`
  - `delegate IList to _items;`
  - Automatically synthesizes forwarding methods on the class for all methods in the contract (`PilotContractRegistry`) that are not explicitly overridden by the class.

### 5.4 Structs, Interfaces, Enums, Actors & Database Models
```ebnf
struct_def    ::= decorators? declaration_modifier* 'struct' identifier generic_params? where_type_clause? ':'? ('{' struct_member* '}' | suite)
interface_def ::= decorators? declaration_modifier* ('interface' | 'protocol') identifier generic_params? where_type_clause? ':'? suite
enum_def      ::= decorators? declaration_modifier* 'enum' identifier generic_params? ':'? '{' enum_member (',' enum_member)* ','? '}'
actor_def     ::= decorators? declaration_modifier* 'actor' identifier generic_params? ':'? suite
model_def     ::= decorators? declaration_modifier* ('model' | 'table' | 'schema') identifier generic_params? ('(' class_bases? ')')? ':'? suite
```

---

## 6. Properties, Operator Overloading, Extensions & Type Adapters

### 6.1 Computed Properties (`property_def`)
```ebnf
property_def ::= decorators? declaration_modifier* 'property' identifier (':' type_spec)? ':'? '{' (get_accessor | set_accessor)* '}'
get_accessor ::= 'get' (':'? suite | ';')?
set_accessor ::= 'set' ('(' identifier ')')? (':'? suite | ';')?
```

### 6.2 Operator Overloading (`operator_def`)
```ebnf
operator_def    ::= decorators? declaration_modifier* 'operator' overloadable_op parameters ('->' type_spec)? ':'? suite
overloadable_op ::= '+' | '-' | '*' | '/' | '%' | '**' | '//'
                  | '==' | '!=' | '<' | '<=' | '>' | '>='
                  | '&' | '|' | '^' | '~' | '<<' | '>>'
                  | '[' ']' '='?
```

### 6.3 Scoped Lexical Extensions (`extension_def`)
```ebnf
extension_def ::= decorators? declaration_modifier* 'extension' identifier? ('for' | 'on') type_spec where_type_clause? ':'? suite
```
Registers a `PilotLexicalExtension` on `type_spec`. Unlike global monkey-patching, methods defined in an `extension` are **inactive by default** and only resolve on target values inside a lexical `with extension <Name> { ... }` block.

### 6.4 Type Adapters (`adapter_def`)
```ebnf
adapter_def ::= decorators? declaration_modifier* 'adapter' (type_spec ('->' | 'to') type_spec | identifier) ':'? suite
```
Registers a conversion function (`self: FromType -> ToType`) in `PilotTypeAdapterRegistry.Default`. Inside the `adapter` body, the source value is bound to `self`. Whenever a function or method group overload expects `ToType` and receives `FromType`, the adapter is invoked automatically.

---

## 7. Control Flow, `guard`, `defer`, `try` & `with` Statements

### 7.1 Conditional & Loop Statements
```ebnf
if_stmt      ::= 'if' test ':'? suite (('elif' | 'else' 'if') test ':'? suite)* ('else' ':'? suite)?
while_stmt   ::= 'while' test ('bounded' test)? ':'? suite ('else' ':'? suite)?
for_stmt     ::= ('async' | 'parallel')? 'for' for_target 'in' test ('bounded' test)? ':'? suite ('else' ':'? suite)?
labeled_stmt ::= identifier ':' (for_stmt | while_stmt | block)
```
- `for_stmt` iterates over `list`, `tuple`, `range`, `set`, `generator`, `str` (yielding `char` values), and `map` (yielding keys).
- `break [label]` and `continue [label]` control loop termination or jump out of nested loops when a `label` is specified.

### 7.2 `guard` Precondition Statement
```ebnf
guard_stmt ::= 'guard' ('let' pattern '=' test | test) 'else' ':'? suite
```
- Evaluates the boolean condition `test` (or attempts to match `pattern` against `test` in `guard let`).
- If the condition is falsy (or pattern match fails), executes the `else` suite (which typically returns, breaks, continues, or raises an error).

### 7.3 `defer` Cleanup Statement
```ebnf
defer_stmt ::= 'defer' (':'? suite | small_stmt | test)
```
- Pushes the deferred action onto the active execution context's LIFO defer stack (`Context.PushDefer`).
- Guaranteed to execute in **reverse registration order (LIFO)** inside the `finally` block of function invocation (`PilotEngine.InvokeFunction`) or program termination (`VisitProgram`), even if an exception or early `return` occurs.

### 7.4 Exception Handling (`try_stmt`)
```ebnf
try_stmt      ::= 'try' ':'? suite except_clause* ('else' ':'? suite)? ('finally' ':'? suite)?
except_clause ::= 'except' ((type_spec ('as' identifier)?) | ('as' identifier))? ':'? suite
```
- **`except` Clauses**: Evaluated in order when a runtime exception occurs (control-flow signals like `return`/`break`/`continue`/`yield` bypass `except`).
  - Catch all with binding: `except as err { ... }` or `except err { ... }` (if `err` is not a known exception type name, Pilot binds the error payload to `err`).
  - Catch specific type: `except DivideByZeroError as err { ... }` or `except TypeError { ... }`.
- **`else` Clause**: Runs only if the `try` block completed without throwing any exception.
- **`finally` Clause**: Always runs before exiting the `try` statement.

### 7.5 Scoped Context Statement (`with_stmt`)
```ebnf
with_stmt ::= 'async'? 'with' with_item (',' with_item)* ':'? suite
with_item ::= 'extension' identifier
            | 'context' identifier '=' test
            | test ('as' identifier)?
```
Supports three specialized runtime modes (which can be combined in a single `with` statement):
1. **`with extension ExtName`**: Pushes `ExtName` onto `Context.ActiveExtensions` for the duration of `suite`, then pops it in `finally`.
2. **`with context name = expr`**: Pushes `{ name: val }` onto `Context.ContextualBindings` for implicit downstream `context` parameter injection and declares `name` in the current scope, popping the frame on exit.
3. **`with auto_repeat` / `with repeat` / `with vectorize`**: Increments `Context.AutoRepeatDepth`, enabling automatic collection-to-scalar call unrolling inside `suite`.

---

## 8. Pattern Matching (`match` & `is`)

Pattern matching is available both as a multi-arm statement (`match_stmt`) and as an inline boolean comparison expression (`expr is pattern` / `expr is not pattern`).

```ebnf
match_stmt          ::= 'match' test ':'? '{' match_arm* '}'
match_arm           ::= 'case' pattern (('when' | 'if') test)? (':' | '->' | '=>')? suite
pattern             ::= or_pattern
or_pattern          ::= and_pattern (('or' | '|') and_pattern)*
and_pattern         ::= not_pattern ('and' not_pattern)*
not_pattern         ::= 'not' not_pattern | primary_pattern
primary_pattern     ::= relational_pattern
                      | collection_pattern
                      | property_pattern
                      | tuple_pattern_match
                      | type_or_var_pattern
                      | literal
                      | symbol_literal
                      | '_'
```

### All Supported Pattern Forms (`PilotPatternMatcher.cs`)

| Pattern Type | Syntax | Match Semantics |
| :--- | :--- | :--- |
| **Wildcard** | `_` | Matches any value (including `null` and `undefined`) without binding a variable. |
| **Literal** | `200`, `"OK"`, `true`, `null`, `:sym` | Matches if `value.Equals(literal)` is `true`. |
| **Relational** | `> 10`, `<= 0`, `== 42`, `!= 0` | Compares `value` against the right-hand expression using `<`, `<=`, `>`, `>=`, `==`, or `!=`. |
| **Disjunction (`\|` / `or`)** | `200 \| 201 \| 204` | Succeeds if any sub-pattern matches. |
| **Conjunction (`and`)** | `> 0 and < 100` | Succeeds only if all sub-patterns match. |
| **Negation (`not`)** | `not null` | Succeeds if the inner pattern does *not* match. |
| **Type Pattern** | `int x`, `str s`, `User u` | Checks if `value` is of the specified built-in or class type and optionally binds it to identifier `x`. |
| **Variable Capture** | `let x`, `var x`, or bare `x` | Matches any value and binds it to `x` in the `MatchArm` scope. |
| **Collection & Slice** | `[1, 2, 3]` or `[first, ..mid, last]` | Matches a `list` or `tuple`. Supports `..` or `*` slice rest patterns at the beginning, middle, or end, binding the captured slice as a `list`. |
| **Tuple** | `(a, b)` | Matches a `tuple` or `list` of the exact arity, recursively matching each element pattern. |
| **Property / Object** | `{ status: 200, role: "admin" }` | Matches a `PilotObject` whose members exist and recursively match each specified pattern. |
| **Guard Clause** | `case x when x % 2 == 0 => ...` | Evaluates the `when` / `if` boolean expression in the arm's scope after the pattern matches; if false, continues to the next `case` arm. |

---

## 9. Relational Database & LINQ/SQL Query Syntax

Pilot embeds both **LINQ-style comprehension queries**, **SQL-first queries**, **indexed lookups**, and **PostgreSQL DML/Transaction statements** directly in the core grammar:

```ebnf
// 1. Database & Transaction Blocks
database_stmt    ::= 'database' identifier (':' type_spec)? '=' test
database_def     ::= 'database' identifier ('(' testlist? ')')? ':'? suite
transaction_stmt ::= 'transaction' ('(' test ')')? ':'? suite
savepoint_stmt   ::= 'savepoint' identifier
savepoint_block  ::= 'savepoint' identifier ':'? suite
commit_stmt      ::= 'commit' 'transaction'?
rollback_stmt    ::= 'rollback' ('to' 'savepoint'? identifier | 'transaction')?

// 2. LINQ-Style Query Expression
query_expr       ::= from_clause join_clause* where_clause? group_clause? order_clause? select_clause limit_clause?
from_clause      ::= 'from' identifier ('as' identifier)? 'in' test index_hint?
join_clause      ::= ('inner' | 'left' | 'right' | 'full' | 'cross')? 'join' identifier ('as' identifier)? ('in' test)? 'on' test
where_clause     ::= 'where' test
group_clause     ::= 'group' test 'by' test ('into' identifier)? ('having' test)?
order_clause     ::= 'order' 'by' ordering_term (',' ordering_term)*
select_clause    ::= 'select' 'distinct'? ('*' | select_item (',' select_item)*)
limit_clause     ::= 'limit' test ('offset' test)?

// 3. SQL-First Query Expression
sql_query_expr   ::= 'select' 'distinct'? select_projection 'from' from_source join_clause* where_clause? group_clause? order_clause? limit_clause?

// 4. Direct Indexed Lookup
lookup_expr      ::= 'lookup' test index_hint? 'where' test
index_clause     ::= 'index' 'by'? '(' index_field (',' index_field)* ')'
index_hint       ::= ('with' | '@') 'index' '(' identifier (',' identifier)* ')'

// 5. DML (Insert / Update / Delete with Upsert & Returning)
insert_stmt      ::= 'insert' 'into' dotted_name ('(' identifier (',' identifier)* ','? ')')?
                     ('values' insert_values_group (',' insert_values_group)* | query_expr | sql_query_expr)
                     on_conflict_clause? returning_clause?
update_stmt      ::= 'update' dotted_name 'set' set_assignment_list ('where' test)? returning_clause?
delete_stmt      ::= 'delete' 'from' dotted_name ('where' test)? returning_clause?
```

---

## 10. Concurrency, Channels & `select` Syntax

```ebnf
spawn_stmt         ::= 'spawn' ('(' test ')')? ':'? suite
atomic_stmt        ::= 'atomic' ('(' test? ')')? ':'? suite
sync_stmt          ::= ('sync' | 'lock') '(' test ')' ':'? suite
isolate_stmt       ::= 'isolate' ':'? suite
channel_send_stmt  ::= test '<-' test
select_stmt        ::= 'select' ':'? '{' select_arm* '}'
select_arm         ::= 'case' select_case_target (':' | '->')? suite
                     | 'timeout' duration_term (':' | '->')? suite
                     | 'default' (':' | '->')? suite
select_case_target ::= identifier '=' '<-' test
                     | test '<-' test
                     | '<-' test
```
- **Unary `spawn expr`**: Evaluates `expr` asynchronously in a background `PilotFuture` task on a cloned child `PilotExecutionContext`.
- **Unary `await expr`**: If `expr` evaluates to a `PilotFuture`, blocks until completion (`future.Await()`) and returns the resulting `PilotValue`.

---

## 11. Real-Time Determinism & Memory Control Syntax

```ebnf
deadline_stmt ::= 'deadline' duration_term ('jitter' duration_term)? ':'? suite ('on_missed' ':'? suite)?
within_stmt   ::= 'within' duration_term ':'? suite ('fallback' ':'? suite)?
periodic_stmt ::= 'periodic' duration_term ('priority' test)? ('jitter' duration_term)? ':'? suite
noalloc_stmt  ::= 'noalloc' ':'? suite
region_stmt   ::= 'region' identifier ':'? suite
unit_stmt     ::= 'unit' identifier ('=' unit_expr)? ';'?
unit_expr     ::= unit_term (('*' | '/' | '**') unit_term)*
```
- **Stack Allocation Expression**: `stackalloc type_spec ('[' test ']')?` (e.g., `let buf = stackalloc u8[1024]`).

---

## 12. Polyglot Interoperability, Macros & UDSA Syntax

### 12.1 Foreign Imports & `extern` Declarations
```ebnf
foreign_import_stmt ::= 'import' foreign_target ('as' identifier)?
                      | 'from' foreign_target 'import' ('*' | import_as_names | '(' import_as_names ')')
foreign_target      ::= (foreign_language | identifier) ':' (dotted_name | STRING)
extern_def          ::= 'extern' (STRING | identifier) (('assembly' | 'module' | 'lib') (STRING | identifier))? ':'? ('{' extern_member* '}' | extern_member)
```

### 12.2 Inline Polyglot Blocks (`inline_foreign_block`)
Can be used both as a standalone statement (`compound_stmt`) and as an expression (`atom`):
```ebnf
inline_foreign_block ::= foreign_language (RAW_DSL_BLOCK | '{' stmt* '}' | TRIPLE_STRING | STRING)
foreign_language     ::= 'python' | 'csharp' | 'rust' | 'js' | 'javascript' | 'java'
                       | 'asm' | 'assembly' | 'cpp' | 'c++' | 'c' | 'lua'
                       | 'sql' | 'postgres' | 'postgresql' | 'mysql'
```
- If an inline block contains `yield`, `VisitInline_foreign_block` compiles it into a **resumable polyglot coroutine function** (`<lang>_coroutine`) that shares variables with the host scope and resumes across `yield` points on each invocation.

### 12.3 Macros, Quoting & Embedded DSLs
```ebnf
macro_def       ::= decorators? declaration_modifier* ('macro' | 'defmacro') '!'? identifier generic_params? parameters? ('->' type_spec)? ':'? suite
macro_call      ::= identifier '!' ( '(' arglist? ')' | '{' stmt* '}' | '[' arglist? ']' )
quote_expr      ::= 'quote' ':'? (suite | '{' test '}' | '(' test ')')
quasiquote_expr ::= ('quasiquote' | '`') ':'? (suite | '{' test '}' | '(' test ')' | identifier | literal)
splice_expr     ::= '$' '@' (identifier | '(' test ')')
                  | ('$' | 'splice' | 'unquote') (identifier | '(' test ')')
                  | 'unquote_splicing' (identifier | '(' test ')')
dsl_stmt        ::= ('dsl' | 'domain') identifier (parameters | '(' dsl_args? ')')? ':'? (suite | RAW_DSL_BLOCK)
```

---

## 13. Expression Hierarchy & Operator Precedence Table

The following table lists all expression operators in [`antlr/Pilot.g4`](../antlr/Pilot.g4) from **highest precedence (1)** to **lowest precedence (17)**:

| Level | Grammar Rule | Operators / Syntax | Associativity | Description |
| :---: | :--- | :--- | :---: | :--- |
| **1** | `atom` | `literal`, `identifier`, `[...]`, `(...)`, `{...}`, `nameof(...)`, `macro!(...)`, `:sym`, `quote`, `python { ... }`, `select ...` | Primary | Literals, identifiers, collection literals, parenthesized expressions, inline polyglot/SQL blocks. |
| **2** | `trailer` | `f(args)` \| `f*(args)` \| `a[i]` \| `a.b` \| `a?.b` \| `a..m()` \| `a?..m()` \| `x<unit>` \| `x!` \| `over (...)` | Left-to-Right | Function calls, broadcast calls (`*()`), indexing/slicing, dot navigation, safe navigation (`?.`), cascade chaining (`..`), unit tagging, non-null assertion (`!`), SQL `over` window clause. |
| **3** | `power` | `a ** b` | Right-to-Left | Exponentiation (`Math.Pow`). |
| **4** | `factor` / `unary_op` | `+x`, `-x`, `~x`, `await x`, `<-ch`, `spawn x`, `stackalloc T[n]`, `quote`, `quasiquote`, `$x` | Right-to-Left | Unary arithmetic/bitwise operators, async `await`, channel receive `<-`, concurrent `spawn`, stack allocation, macro quoting/splicing. |
| **5** | `term` | `*`, `/`, `//`, `%`, `->`, `->>` | Left-to-Right | Multiplication, division, integer floor division (`//`), modulo (`%`), **structural DTO projection (`->`)**, JSON text extract (`->>`). |
| **6** | `arith_expr` | `+`, `-` | Left-to-Right | Addition, string/list concatenation (`+`), subtraction, **algebraic class subtraction (`-`)**. |
| **7** | `shift_expr` | `<<`, `>>` | Left-to-Right | Bitwise left shift and arithmetic right shift. |
| **8** | `bitand_expr` | `&` | Left-to-Right | Bitwise AND, boolean AND, **algebraic class intersection (`&`)**. |
| **9** | `bitxor_expr` | `^` | Left-to-Right | Bitwise XOR. |
| **10** | `bitor_expr` | `\|` | Left-to-Right | Bitwise OR, boolean OR, **method group union/fallback (`\|`)**, **algebraic class union (`\|`)**. |
| **11** | `range_expr` | `a .. b`, `a ..= b` | Non-Associative | Half-open (`..`) and inclusive (`..=`) integer range construction (`PilotRange`). |
| **12** | `comparison` | `==`, `!=`, `<>`, `<`, `<=`, `>`, `>=`, `in`, `not in`, `is`, `is not`, `@>`, `<@` | Chained / Left | Value comparisons, collection membership (`in`), **pattern matching test (`is` / `is not`)**, JSONB containment. |
| **13** | `not_test` | `not x` | Right-to-Left | Logical boolean negation (`!x.IsTrue`). |
| **14** | `and_test` | `and`, `&&` | Left-to-Right | Short-circuiting logical AND. |
| **15** | `or_test` | `or`, `\|\|` | Left-to-Right | Short-circuiting logical OR. |
| **16** | `coalesce_expr` | `a ?? b` | Left-to-Right | Null/undefined coalescing (evaluates `b` only if `a` is `null` or `undefined`). |
| **17** | `set_query_expr` | `union`, `union all`, `intersect`, `except`, `difference` | Left-to-Right | Relational dataset set operations. |
| **18** | `pipe_expr` | `a \|> f`, `f <\| a` | Left-to-Right | Forward and backward functional pipelining. |
| **19** | `test` | `expr if cond else alt`, `lambda_expr` | Right-to-Left | Ternary conditional expression and anonymous lambda definitions. |

---

[**← Lexical Reference**](lexical-reference.md) | [**Documentation Hub**](index.md) | [**Next: Functions & API Reference →**](functions-and-apis.md)
