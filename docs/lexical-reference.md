# Pilot Lexical Reference

[**← Documentation Hub**](index.md) | **Lexical Reference** | [**Syntax Reference →**](syntax-reference.md) | [**Functions & API Reference**](functions-and-apis.md)

---

This reference provides a complete specification of the lexical structure of the Pilot programming language as implemented in the native compiler pipeline (`pilot-lexer`, `pilot-syntax`, `pilot-parser`).

## Table of Contents

1. [Compilation Pipeline & Character Encoding](#1-compilation-pipeline--character-encoding)
2. [Comments, Whitespace & Line Joining](#2-comments-whitespace--line-joining)
3. [Preprocessor Directives & Diagnostics](#3-preprocessor-directives--diagnostics)
4. [Identifiers & Contextual Keywords](#4-identifiers--contextual-keywords)
5. [Complete Keyword Reference (By Category)](#5-complete-keyword-reference-by-category)
6. [Built-in Value Literals](#6-built-in-value-literals)
7. [Numeric Literals](#7-numeric-literals)
8. [Dimensional & Real-Time Literals](#8-dimensional--real-time-literals)
9. [String, Interpolated, Symbol & Raw DSL Literals](#9-string-interpolated-symbol--raw-dsl-literals)
10. [Operators & Delimiters](#10-operators--delimiters)

---

## 1. Compilation Pipeline & Character Encoding

Pilot source files (`.pilot`) are encoded in **UTF-8**. The source text is tokenized into zero-copy lexical tokens by [`pilot-lexer`](../crates/pilot-lexer/src/lib.rs) with full position tracking and trivia preservation while maintaining strict line-number mapping.

---

## 2. Comments, Whitespace & Line Joining

All whitespace, comments, and explicit line-continuation sequences are routed to the `HIDDEN` lexer channel and ignored by the parser.

| Token Name | Regex / Pattern | Description & Example |
| :--- | :--- | :--- |
| `WS` | `[ \t\f\r\n]+` | Spaces, horizontal tabs, form feeds, carriage returns, and newlines. Note: Pilot uses explicit braces `{ ... }` or colons `:` for blocks, so indentation is not syntactically significant. |
| `COMMENT` | `('#' \| '//') ~[\r\n]*` | Single-line comment starting with either Python-style `#` or C#-style `//` up to the end of the line. |
| `BLOCK_COMMENT` | `'/*' .*? '*/'` | Multi-line C-style block comment spanning across lines. |
| `LINE_JOINING` | `'\\' '\r'? '\n'` | Backslash at the end of a physical line joins it with the next physical line. |
| `Shebang` | `^#!.*` (Line 1 only) | UNIX interpreter directive on line 1 (e.g., `#!/usr/bin/env pilot`), stripped by `PilotPreprocessor`. |

---

## 3. Preprocessor Directives & Diagnostics

Preprocessor directives begin with `#` (with optional leading whitespace) and are evaluated prior to AST construction.

### 3.1 Supported Directives

| Directive | Syntax | Exact Behavior |
| :--- | :--- | :--- |
| `#define` | `#define SYMBOL` | Defines a compilation symbol in `DefinedSymbols` if the current conditional branch is active. Symbol names must match `^[a-zA-Z_][a-zA-Z0-9_]*$`. |
| `#undef` | `#undef SYMBOL` | Removes `SYMBOL` from `DefinedSymbols` if the current conditional branch is active. |
| `#if` | `#if <condition>` | Opens a conditional compilation block. Evaluates `<condition>` using boolean operators (`&&`, `\|\|`, `!`, `and`, `or`, `not`, `==`, `!=`, parentheses `(...)`, `true`, `false`, and symbol identifiers). |
| `#elif` | `#elif <condition>` | Evaluates an alternative branch if no preceding `#if` or `#elif` branch in the same frame was taken. Cannot appear after `#else`. |
| `#else` | `#else` | Fallback conditional branch executed when all preceding `#if`/`#elif` branches evaluated to `false`. Only one `#else` is allowed per `#if`. |
| `#endif` | `#endif` | Closes the innermost open `#if` conditional block. |
| `#region` | `#region [Name]` | Marks the start of a collapsible code region recorded in `PilotPreprocessResult.Regions`. Defaults to `"Region"` if unnamed. |
| `#endregion` | `#endregion [Name]` | Closes the innermost open `#region`. |
| `#error` | `#error "Message"` | Emits a fatal preprocessor diagnostic (`PREPROC_ERR`) if reached inside an active branch, halting execution/compilation. |
| `#warning` | `#warning "Message"` | Emits a warning diagnostic (`PREPROC_WARN`) if reached inside an active branch. |
| `#pragma` | `#pragma warning disable\|restore ID1, ID2` | Suppresses or restores specific compiler/analyzer warning codes (e.g., `PILOT101`) across line ranges. |
| `#nullable` | `#nullable enable\|disable\|restore` | Standard metadata pragma tracked for nullability context. |
| `#line` | `#line <number> ["file"]` | Standard metadata pragma for line mapping. |

### 3.2 Preprocessor Diagnostic Codes

| Code | Severity | Condition |
| :--- | :--- | :--- |
| `PREPROC001` | Error | End-of-file reached with an unclosed `#if` block (missing `#endif`). |
| `PREPROC002` | Warning | `#region` was not closed with a matching `#endregion` before EOF. |
| `PREPROC003` | Error | Invalid identifier supplied to `#define`. |
| `PREPROC004` | Error | Invalid identifier supplied to `#undef`. |
| `PREPROC005` | Error | Unexpected `#elif` without a preceding `#if`. |
| `PREPROC006` | Error | `#elif` appeared after an `#else` directive in the same block. |
| `PREPROC007` | Error | Unexpected `#else` without a preceding `#if`. |
| `PREPROC008` | Error | Multiple `#else` directives in the same `#if` block. |
| `PREPROC009` | Error | Unexpected `#endif` without a matching `#if`. |
| `PREPROC010` | Warning | Unexpected `#endregion` without a matching `#region`. |
| `PREPROC011` | Error | Syntax error in a `#if` or `#elif` boolean condition expression. |
| `PREPROC_ERR` | Error | User-triggered `#error` directive in an active branch. |
| `PREPROC_WARN` | Warning | User-triggered `#warning` directive in an active branch. |

---

## 4. Identifiers & Contextual Keywords

### 4.1 `NAME` Token
Standard identifiers match the regular expression:
```regex
[a-zA-Z_][a-zA-Z0-9_]*
```
Identifiers begin with an ASCII letter (`a`–`z`, `A`–`Z`) or underscore (`_`), followed by zero or more letters, digits (`0`–`9`), or underscores.

### 4.2 Contextual Keyword Resolution (`identifier` Rule)
Because Pilot spans multiple domains (relational queries, real-time scheduling, concurrency, polyglot FFI, and metaprogramming), the parser rule `identifier` (lines 23–47 of [`Pilot.g4`](../antlr/Pilot.g4)) permits domain-specific keywords to also function as ordinary variable, field, parameter, and function names when used in identifier positions.

**Strict Reserved Words** (tokens that are *not* part of the `identifier` fallback rule and cannot be used as variable names):
- Control flow: `if`, `elif`, `else`, `while`, `for`, `in`, `return`, `yield`, `break`, `continue`, `pass`, `raise`, `try`, `finally`, `with`, `as`, `assert`, `import`, `global`, `lambda`
- Logical & identity operators: `and`, `or`, `not`, `is`
- Type/Object declarations: `def`, `class`, `struct`, `interface`, `protocol`, `enum`
- Literals: `true`, `True`, `false`, `False`, `null`, `none`, `None`

All other keyword tokens listed in Section 5 can be used both for their syntactic construct and as contextual identifiers.

---

## 5. Complete Keyword Reference (By Category)

### 5.1 Declarations, Types & Access Modifiers

| Keyword | Token | Exact Meaning & Usage |
| :--- | :--- | :--- |
| `public` | `PUBLIC` | Access modifier declaring a member, function, or type as publicly accessible across modules. |
| `private` | `PRIVATE` | Access modifier restricting access to the declaring class or struct. |
| `protected` | `PROTECTED` | Access modifier restricting access to the declaring class and its subclasses. |
| `internal` | `INTERNAL` | Access modifier restricting access to the current package/assembly. |
| `static` | `STATIC` | Declares a member or function as belonging to the type rather than an instance. |
| `partial` | `PARTIAL` | Allows splitting a class, struct, or interface definition across multiple declarations; merged by `VisitClass_def`. |
| `let` | `LET` | Declares a local variable or class field binding (`let x: int = 10`), or binds a variable inside a `guard let` / `match` pattern. |
| `var` | `VAR` | Declares a mutable variable or indexed in-memory table binding (`var users = ... index by (...)`). |
| `const` | `CONST` | Declares an immutable constant binding (`isConst: true`). Reassigning a `const` variable throws a runtime error. |
| `type` | `TYPE` | Declares a type alias (`type UserId = int` or `type Result[T] = T \| Error`). |
| `ref` | `REF` | Declares a by-reference variable, parameter, return type, or lambda capture (`[ref x]`). |
| `dynamic` | `DYNAMIC` | Declares a dynamically typed variable or parameter that bypasses static type constraints. |
| `symbol` | `SYMBOL` | Represents the interned symbol type or constructs a symbol literal (`symbol("alpha")`). |
| `pure` | `PURE` | Function modifier indicating deterministic, side-effect-free execution (`pure def add(a, b)`). |
| `mut` | `MUT` | Mutability specifier keyword. |
| `immut` | `IMMUT` | Immutability specifier keyword. |
| `def` | `DEF` | Python-style function or method declaration keyword (`def calc(x: int) -> int`). |
| `fn` | `FN` | Rust-style function or lambda declaration keyword (`fn calc(x: int) -> int`, completely interchangeable with `def`). |
| `class` | `CLASS` | Declares a reference-type class blueprint, or an algebraic class composition (`class Admin = User \| Roles`). |
| `struct` | `STRUCT` | Declares an unboxed value-type structure for zero-GC / hard real-time workloads. |
| `interface` | `INTERFACE` | Declares a nominal or structural interface contract. |
| `protocol` | `PROTOCOL` | Alias for `interface` (Python/Swift-style protocol declaration). |
| `enum` | `ENUM` | Declares an enumeration or algebraic sum type (`enum Status { Ok, Error(str) }`). |
| `property` | `PROPERTY` | Declares a C#-style computed property with `get` and `set` accessors. |
| `get` | `GET` | Defines the getter accessor inside a `property` body. |
| `set` | `SET_KW` | Defines the setter accessor inside a `property` body (`set(val) { ... }`), or specifies column assignments in SQL `UPDATE ... SET`. |
| `operator` | `OPERATOR` | Declares a custom operator overload on a class or struct (`operator +(self, other)`). |
| `extension` | `EXTENSION` | Declares a lexically scoped extension (`extension Name on Type { ... }`) or activates one (`with extension Name`). |
| `state` | `STATE` | Declares a typestate block inside a class (`state Connected { ... }`). The first declared state becomes `InitialState`. |
| `adapter` | `ADAPTER` | Declares a type adapter conversion rule (`adapter str -> int { return int(self); }`). |
| `converter` | `CONVERTER` | Contextual keyword alias for type converter definitions. |
| `steal` | `STEAL` | Copies methods from a non-inherited class into the current class (`steal EncryptionEngine` or `steal from Ops [exp]`). |
| `delegate` | `DELEGATE` | Forwards all methods of an interface/contract to a member field or expression (`delegate IList to self._items`). |
| `contracts` | `CONTRACTS` | Binds contract forwarding for a type (`contracts for IList = _items` or `IList.Contracts = _items`). |
| `params` | `PARAMS` | Marks a parameter as a variadic parameter list collecting remaining positional arguments into a `list`. |
| `args` | `ARGS` | Contextual keyword for positional argument collections and the global CLI arguments list. |
| `kwargs` | `KWARGS` | Contextual keyword for keyword argument maps. |
| `nameof` | `NAMEOF` | Compile-time operator returning the string name of an identifier, dotted path, or expression (`nameof(user.id)` -> `"user.id"`). |
| `each` | `EACH` | Parameter or call-site modifier that unrolls a collection argument into repeated scalar invocations (`square(each [1, 2, 3])`). |
| `repeat` | `REPEAT` | Synonym for `each` at call sites/parameters, or activates auto-vectorization in `with repeat { ... }`. |

### 5.2 Control Flow, Preconditions, Contracts & Pattern Matching

| Keyword | Token | Exact Meaning & Usage |
| :--- | :--- | :--- |
| `if` | `IF` | Conditional branch statement (`if cond { ... }`), ternary expression (`a if cond else b`), comprehension filter, or match guard. |
| `elif` | `ELIF` | Else-if branch in an `if` statement (`else if` is also supported directly). |
| `else` | `ELSE` | Fallback branch in `if`, `while`, `for`, `try`, `guard`, and ternary expressions. |
| `while` | `WHILE` | Conditional loop (`while cond { ... }`), optionally with a `bounded N` iteration cap and `else` completion block. |
| `for` | `FOR` | Iterator loop (`for item in iterable { ... }`), supports `async for`, `parallel for`, `bounded N`, and destructuring targets. |
| `in` | `IN` | Membership test operator (`x in list`) and iteration target separator (`for x in range(10)`). |
| `match` | `MATCH` | Structural, relational, and type pattern matching statement (`match val { case ... }`). |
| `case` | `CASE` | Defines a branch arm inside a `match` or concurrency `select` statement. |
| `when` | `WHEN` | Pattern guard condition on a `match` arm (`case x when x > 0 => ...`). |
| `default` | `DEFAULT` | Non-blocking fallback arm in a concurrency `select` block. |
| `timeout` | `TIMEOUT` | Timed expiration arm in a concurrency `select` block (`timeout 500ms => ...`). |
| `guard` | `GUARD` | Mandatory early-exit precondition (`guard cond else { return ... }` or `guard let pat = expr else { ... }`). |
| `defer` | `DEFER` | Schedules a statement or block to execute in **LIFO (Last-In, First-Out)** order when the enclosing function or program exits. |
| `context` | `CONTEXT` | Marks a function parameter for implicit dependency injection (`context db: Database`) or binds a context value (`with context db = ...`). |
| `requires` | `REQUIRES` | Design-by-Contract precondition evaluated before function body execution; throws `PilotContractException` (`ContractError`) if false. |
| `ensures` | `ENSURES` | Design-by-Contract postcondition evaluated after function return; has access to `result` and `old(expr)` snapshots. |
| `return` | `RETURN` | Exits the current function and returns a value (or tuple of values if comma-separated). |
| `yield` | `YIELD` | Suspends a generator function or polyglot coroutine and produces a value to the caller. |
| `break` | `BREAK` | Exits the innermost loop, or exits a specific outer labeled loop/block (`break outer_loop`). |
| `continue` | `CONTINUE` | Skips to the next iteration of the innermost (or labeled) loop. |
| `pass` | `PASS` | No-operation placeholder statement for empty blocks. |
| `raise` | `RAISE` | Throws a `PilotUserException` (`raise "Error"` or `raise CustomError("msg") from cause`). |
| `try` | `TRY` | Opens a protected exception-handling block. |
| `except` | `EXCEPT` | Catches exceptions in a `try` block (`except TypeError as e { ... }`), or computes set difference in database queries. |
| `catch` | `CATCH` | Catches exceptions in a `try` block (`catch (e) { ... }` or `catch { ... }`). |
| `finally` | `FINALLY` | Block in a `try` statement guaranteed to execute regardless of whether an exception or return occurred. |
| `with` | `WITH` | Scoped context manager for lexical extensions (`with extension E`), DI bindings (`with context k = v`), `with auto_repeat`, or resources. |
| `as` | `AS` | Binds an alias in `import`, `except`, `with`, and SQL/LINQ `SELECT` / `FROM` / `JOIN` clauses. |
| `assert` | `ASSERT` | Statement (`assert cond, "msg"`) that throws `PilotAssertionException` (`AssertionError`) if `cond` is falsy. |
| `import` | `IMPORT` | Imports native Pilot modules or foreign polyglot packages (`import python:math as m`). |
| `global` | `GLOBAL` | Declares that listed identifiers inside a function refer to bindings in the global scope. |
| `lambda` | `LAMBDA` | Python-style anonymous function expression (`lambda x, y: x + y`). |
| `and` | `AND` | Short-circuiting logical AND (also matches `&&`) and pattern conjunction (`pat1 and pat2`). |
| `or` | `OR` | Short-circuiting logical OR (also matches `\|\|`) and pattern disjunction (`pat1 or pat2`). |
| `not` | `NOT` | Logical negation (`not x`), negated membership (`x not in list`), negated identity (`x is not null`), and negated pattern (`not pat`). |
| `is` | `IS` | Pattern & identity matching operator (`x is int`, `x is > 10`, `x is not null`). |

### 5.3 Relational & In-Memory Database Keywords

| Keyword | Token | Exact Meaning & Usage |
| :--- | :--- | :--- |
| `database` | `DATABASE` | Declares an in-memory or connected database scope (`database Store { ... }` or `database db = connect(...)`). |
| `model` / `table` / `schema` | `MODEL` / `TABLE` / `SCHEMA` | Declares a relational table or entity model schema (`table users (id int, name str)`). |
| `index` / `indexed` | `INDEX` / `INDEXED` | Declares in-memory indexes (`index by (id [hash, unique], email [btree])`) or query hints (`with index(id)`). |
| `lookup` | `LOOKUP` | Performs a direct $O(1)$ / $O(\log n)$ indexed key lookup (`lookup users with index(id) where id == 42`). |
| `from` | `FROM` | Specifies the data source in LINQ queries (`from u in users`), SQL queries (`select * from users`), `delete from`, `import`, `steal from`, and `raise ... from`. |
| `select` | `SELECT` | Projects output columns in LINQ/SQL queries (`select u.name as username`) or multiplexes channels (`select { case ... }`). |
| `distinct` | `DISTINCT` | Eliminates duplicate rows in a `select distinct` projection. |
| `where` | `WHERE` | Filters rows in queries/updates/deletes (`where u.active == true`) or specifies generic type constraints (`where T: IComparable`). |
| `join` | `JOIN` | Joins two relations on a condition (`join orders as o in orders_list on u.id == o.user_id`). |
| `inner`, `left`, `right`, `full`, `cross`, `outer` | `INNER`, `LEFT`, `RIGHT`, `FULL`, `CROSS`, `OUTER` | Join type specifiers before `join`. |
| `on` | `ON` | Specifies join predicates (`on a.id == b.id`), conflict targets (`on conflict (id)`), or extension targets (`extension E on str`). |
| `group`, `by`, `into`, `having` | `GROUP`, `BY`, `INTO`, `HAVING` | Groups query rows (`group u by u.dept into g having g.count() > 5`). |
| `order`, `asc`, `desc`, `ascending`, `descending` | `ORDER`, `ASC`, `DESC`, `ASCENDING`, `DESCENDING` | Sorts query results (`order by u.created_at descending`). |
| `limit`, `offset` | `LIMIT`, `OFFSET` | Paginates query results (`limit 25 offset 50`). |
| `union`, `all`, `intersect`, `difference` | `UNION`, `ALL`, `INTERSECT`, `DIFFERENCE` | Set-theoretic operators combining relations (`q1 union all q2`, `q1 intersect q2`, `q1 difference q2`). |
| `insert`, `values` | `INSERT`, `VALUES` | Postgres-style row insertion (`insert into users (id, name) values (1, "Ada")`). |
| `update` | `UPDATE` | Postgres-style row update (`update users set active = false where id == 1`) or `on conflict do update set ...`. |
| `delete` | `DELETE` | Postgres-style row deletion (`delete from users where active == false`). |
| `conflict`, `do`, `nothing` | `CONFLICT`, `DO`, `NOTHING` | Upsert conflict resolution (`on conflict (id) do nothing` or `on conflict (id) do update set ...`). |
| `returning` | `RETURNING` | Returns projected columns from `insert`, `update`, or `delete` statements (`returning id, created_at`). |
| `over`, `partition`, `window` | `OVER`, `PARTITION`, `WINDOW` | SQL window function specifications (`sum(salary) over (partition by dept order by id asc)`). |
| `transaction`, `commit`, `rollback`, `savepoint`, `to`, `connect` | `TRANSACTION`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`, `TO`, `CONNECT` | ACID transaction blocks (`transaction { ... }`), savepoints (`savepoint sp1`), commits (`commit`), and rollbacks (`rollback to savepoint sp1`). |

### 5.4 Concurrency, Channels & Actors

| Keyword | Token | Exact Meaning & Usage |
| :--- | :--- | :--- |
| `async` | `ASYNC` | Marks a function (`async def`), `for` loop (`async for`), or `with` block (`async with`) as asynchronous. |
| `await` | `AWAIT` | Unary prefix operator (`await future`) that suspends/waits until a `PilotFuture` completes and unwraps its result. |
| `spawn` | `SPAWN` | Launches a concurrent task on the thread pool as a statement (`spawn { ... }`), expression (`spawn compute(x)`), or built-in call, returning a `Future`. |
| `parallel` | `PARALLEL` | Loop modifier (`parallel for item in list { ... }`) for data-parallel iteration. |
| `actor` | `ACTOR` | Declares an isolated concurrent actor type (`actor Worker { ... }`). |
| `chan` / `channel` | `CHAN` / `CHANNEL` | Constructs or annotates a concurrent message channel (`let ch = channel(10)`). |
| `atomic` | `ATOMIC` | Executes a suite atomically (`atomic { counter += 1; }`). |
| `sync` / `lock` | `SYNC` / `LOCK` | Acquires a mutual-exclusion monitor lock on a target object for the duration of the block (`lock(mutex) { ... }`). |
| `isolate` | `ISOLATE` | Executes a block in an isolated memory/execution sandbox (`isolate { ... }`). |

### 5.5 Real-Time Determinism & Memory Control

| Keyword | Token | Exact Meaning & Usage |
| :--- | :--- | :--- |
| `realtime`, `hard_rt`, `soft_rt` | `REALTIME`, `HARD_RT`, `SOFT_RT` | Real-time execution mode specifiers and decorator identifiers. |
| `wcet` | `WCET` | Worst-Case Execution Time annotation (`@wcet(50us)`). |
| `deadline` | `DEADLINE` | Enforces a time budget on a block (`deadline 5ms jitter 100us { ... } on_missed { ... }`). |
| `on_missed` | `ON_MISSED` | Handler block attached to a `deadline` statement triggered if the deadline is exceeded. |
| `within` | `WITHIN` | Soft real-time time-bounded block (`within 10ms { ... } fallback { ... }`). |
| `fallback` | `FALLBACK` | Degraded-mode recovery block executed when a `within` time budget expires. |
| `periodic` / `period` | `PERIODIC` / `PERIOD` | Schedules a deterministic periodic loop (`periodic 10ms priority 1 jitter 50us { ... }`). |
| `priority` | `PRIORITY` | Specifies real-time scheduling priority on a `periodic` block. |
| `jitter` | `JITTER` | Specifies maximum permissible timing jitter on a `deadline` or `periodic` block. |
| `bounded` | `BOUNDED` | Caps maximum loop iterations in `while` and `for` loops (`while cond bounded 1000 { ... }`) to guarantee termination in WCET analysis. |
| `noalloc` | `NOALLOC` | Enforces zero heap/GC allocations inside the enclosed block (`noalloc { ... }`). |
| `stackalloc` | `STACKALLOC` | Allocates a fixed buffer directly on the call stack (`stackalloc u8[256]`). |
| `region` | `REGION` | Executes a block inside a deterministic arena memory region (`region FrameArena { ... }`). |

### 5.6 Polyglot Interoperability & FFI

| Keyword | Token | Target Ecosystem |
| :--- | :--- | :--- |
| `extern` | `EXTERN` | Declares foreign function/type bindings (`extern "C" lib "m" { ... }`). |
| `module` / `lib` / `assembly` | `MODULE` / `LIB` / `ASSEMBLY` | Specifies the external module, shared library, or .NET assembly in `extern` declarations; `assembly` also starts an inline x86/x64 assembly block. |
| `python` | `PYTHON` | Inline Python block (`python { ... }`) or foreign import (`import python:math`). |
| `csharp` | `CSHARP` | Inline C# / .NET block (`csharp { ... }`) or foreign import (`import csharp:System`). |
| `rust` | `RUST` | Inline Rust block (`rust { ... }`) or foreign import (`import rust:serde`). |
| `js` / `javascript` | `JS` / `JAVASCRIPT` | Inline JavaScript block (`js { ... }`). |
| `java` | `JAVA` | Inline Java block (`java { ... }`). |
| `lua` | `LUA` | Inline Lua block (`lua { ... }`). |
| `c` | `C_KW` | Inline C block (`c { ... }`). |
| `cpp` / `c++` | `CPP` | Inline C++ block (`cpp { ... }` or `c++ { ... }`). |
| `asm` / `assembly` | `ASM` / `ASSEMBLY` | Inline Assembly block (`asm { mov eax, 42; ret; }`). |
| `sql` / `postgres` / `postgresql` | `SQL_KW` / `POSTGRES` / `POSTGRESQL` | Inline PostgreSQL query block (`postgresql { select ... }`) returning structured `PilotObject` rows. |
| `mysql` | `MYSQL` | Inline MySQL query block (`mysql { select ... }`) returning structured `PilotObject` rows. |

### 5.7 Universal Domain-Specific Abstractions (UDSA) & Metaprogramming

| Keyword | Token | Exact Meaning & Usage |
| :--- | :--- | :--- |
| `dsl` / `domain` | `DSL` / `DOMAIN` | Defines or invokes an embedded Domain-Specific Language block (`dsl Router(port=8080) { ... }`). |
| `unit` | `UNIT` | Defines a dimensional unit of measure (`unit N = kg * m / s**2;`). |
| `macro` / `defmacro` | `MACRO` / `DEFMACRO` | Defines a compile-time AST macro (`macro log_expr(e) { ... }` or class generative macro). |
| `quote` | `QUOTE` | Captures an expression or statement suite as an unevaluated AST string/representation (`quote { x + 1 }`). |
| `quasiquote` | `QUASIQUOTE` | Captures an AST template supporting internal unquoting/splicing (also written with backquote `` ` ``). |
| `unquote` / `splice` | `UNQUOTE` / `SPLICE` | Evaluates and splices an expression inside a quasiquoted template (also written `$expr`). |
| `unquote_splicing` | `UNQUOTE_SPLICING` | Splices a list of expressions into a quasiquoted template (also written `$@expr`). |

---

## 6. Built-in Value Literals

| Literal Token | Recognized Lexemes | Runtime `PilotValue` | Truthiness (`IsTrue`) |
| :--- | :--- | :--- | :--- |
| `TRUE` | `true`, `True` | `PilotValue.True` (`Kind = Bool`) | `true` |
| `FALSE` | `false`, `False` | `PilotValue.False` (`Kind = Bool`) | `false` |
| `NULL` | `null` | `PilotValue.Null` (`Kind = Null`) | `false` |
| `NONE` | `none`, `None` | `PilotValue.Null` (`Kind = Null`) | `false` |
| `UNDEFINED` | `undefined` | `PilotValue.Undefined` (`Kind = Undefined`) | `false` |

---

## 7. Numeric Literals

All integer and floating-point literals allow visual underscore separators (`_`) between digits.

| Token | Pattern | Examples | Runtime Representation |
| :--- | :--- | :--- | :--- |
| `INTEGER` | `[0-9][0-9_]*` | `0`, `42`, `1_000_000` | `PilotValueKind.Int` (64-bit signed `long`) |
| `HEX_INTEGER` | `'0' [xX] [0-9a-fA-F_]+` | `0xFF`, `0xdead_beef` | `PilotValueKind.Int` (parsed base-16) |
| `BIN_INTEGER` | `'0' [bB] [01_]+` | `0b1010`, `0b1111_0000` | `PilotValueKind.Int` (parsed base-2) |
| `OCT_INTEGER` | `'0' [oO] [0-7_]+` | `0o755`, `0o644` | `PilotValueKind.Int` (parsed base-8) |
| `FLOAT_NUMBER` | `[0-9]+ '.' [0-9]+ ([eE] [+-]? [0-9]+)?` or `[0-9]+ [eE] [+-]? [0-9]+` | `3.14159`, `2.5e-4`, `1e9` | `PilotValueKind.Float` (64-bit IEEE 754 `double`) |

---

## 8. Dimensional & Real-Time Literals

Pilot natively tokenizes physical time durations, frequencies, and memory sizes at the lexical level without requiring spaces or function calls:

| Token | Pattern | Supported Suffixes | Examples |
| :--- | :--- | :--- | :--- |
| `DURATION_LITERAL` | `[0-9]+ ('.' [0-9]+)? ('ns' \| 'us' \| 'ms' \| 's' \| 'min' \| 'h')` | `ns` (nanoseconds), `us` (microseconds), `ms` (milliseconds), `s` (seconds), `min` (minutes), `h` (hours) | `25ns`, `250us`, `10ms`, `1.5s`, `5min`, `2h` |
| `FREQUENCY_LITERAL` | `[0-9]+ ('.' [0-9]+)? ('Hz' \| 'kHz' \| 'MHz' \| 'GHz')` | `Hz` (Hertz), `kHz` (kilohertz), `MHz` (megahertz), `GHz` (gigahertz) | `60Hz`, `50kHz`, `100MHz`, `2.4GHz` |
| `MEMORY_LITERAL` | `[0-9]+ ('B' \| 'KB' \| 'MB' \| 'GB' \| 'TB')` | `B` (bytes), `KB` (kilobytes), `MB` (megabytes), `GB` (gigabytes), `TB` (terabytes) | `512B`, `64KB`, `16MB`, `4GB`, `1TB` |

Additionally, arbitrary dimensional units can be attached to any expression via angle brackets (`100 <m/s>`) or square brackets (`100 [m]`).

---

## 9. String, Interpolated, Symbol & Raw DSL Literals

| Token / Rule | Syntax | Description & Escape/Interpolation Behavior |
| :--- | :--- | :--- |
| `STRING` | `"..."` or `'...'` | Single-line string literal. Supports escape sequences `\n`, `\r`, `\t`, `\"`, and `\\`. Adjacent string literals (`"a" "b"`) are concatenated by grammar rule `STRING+`. |
| `TRIPLE_STRING` | `"""..."""` or `'''...'''` | Multi-line string literal preserving embedded newlines and quotes without escaping. Can also be used as the body of an inline polyglot block (`python """..."""`). |
| `FSTRING` | `f"..."`, `F'...'`, `f"""..."""` | Interpolated format string. Any `{expr}` inside the string is lexed and parsed on-the-fly by `PilotParser.test()` in the active scope and converted to string (`f"Sum: {a + b}"`). |
| `RAW_DSL_BLOCK` | ```` ```lang ... ``` ```` | Markdown-style fenced raw code block for embedded DSLs or polyglot blocks. Strips the opening fence line and closing ```` ``` ````. |
| `symbol_literal` | `:identifier` or `symbol("name")` | Interned symbol literal (e.g., `:ok`, `:error`, `symbol("token")`). |

---

## 10. Operators & Delimiters

### 10.1 Multi-Character Operators

| Operator | Token Name | Category | Exact Behavior |
| :--- | :--- | :--- | :--- |
| `->` | `ARROW` | Type / Projection / SQL | 1. Function return type (`def f() -> int`).<br>2. Type adapter target (`adapter str -> int`).<br>3. Structural DTO projection (`obj -> { name, role }`) returning a `map` of selected fields.<br>4. Cascade result binding (`obj..compute() -> res`).<br>5. Postgres JSON field extract. |
| `=>` | `FAT_ARROW` | Lambda / Match | 1. Lambda body separator (`x => x * 2`).<br>2. `match` and `select` case arm separator (`case 200 => "OK"`). |
| `<-` | `ARROW_LEFT` | Concurrency | Channel send (`ch <- msg`) and unary channel receive (`<-ch` or `case msg = <-ch`). |
| `\|>` | `PIPE_FORWARD` | Pipeline | Forward pipeline operator (`val \|> fn`). Evaluates `val` and passes it as the argument to `fn(val)`. |
| `<\|` | `PIPE_BACKWARD` | Pipeline | Backward pipeline operator (`fn <\| val`). |
| `?.` | `SAFE_NAV` | Null Safety | Safe member navigation (`obj?.prop`). Evaluates to `undefined` instead of throwing `PilotNullReferenceException` when `obj` is `null` or `undefined`. |
| `..` | `DOTDOT` | Cascade / Range / Spread | 1. **Cascade operator** (`obj..set_a(1)..set_b(2)`): invokes method or assigns property on `obj` and returns `obj`.<br>2. **Half-open range** (`0..5` produces `0, 1, 2, 3, 4`).<br>3. **Collection/Call spread** (`[1, ..items]`, `{..map}`, `fn(..args)`).<br>4. **Pattern slice rest** (`[head, ..tail] = list`). |
| `?..` | `SAFE_CASCADE` | Null Safety + Cascade | Null-conditional cascade chaining (`obj?..set_a(1)`). |
| `..=` | `DOTDOT_EQUAL` | Range | Inclusive range operator (`0..=5` produces `0, 1, 2, 3, 4, 5`). |
| `??` | `COALESCE` | Null Coalescing | Returns the left operand if it is neither `null` nor `undefined`; otherwise evaluates and returns the right operand. |
| `??=` | `COALESCE_EQUAL` | Assignment | Null-coalescing assignment (`x ??= default_val`). |
| `:=` | `WALRUS` | Assignment | Walrus assignment operator in expressions and cascades (`obj..let res := compute()`). |
| `**` | `DOUBLESTAR` | Arithmetic / Unpack | 1. Exponentiation (`2 ** 10` -> `1024.0`).<br>2. Unit exponent (`s**2`).<br>3. Keyword argument / record rest unpacking (`**kwargs`, `{ id, **rest } = rec`). |
| `//` | `DOUBLESLASH` | Arithmetic | Floor / integer division (`7 // 2` -> `3`). Throws `PilotDivideByZeroException` on `0`. |
| `==` | `EQEQUAL` | Comparison | Value/structural equality across primitives, strings, lists, tuples, maps, sets, matrices, and objects. Numeric types (`int`/`float`) compare within `1e-9` tolerance. |
| `!=` or `<>` | `NOTEQUAL` | Comparison | Value inequality (supports both C-style `!=` and SQL-style `<>`). |
| `<=`, `>=` | `LESSEQUAL`, `GREATEREQUAL` | Comparison | Less-than-or-equal and greater-than-or-equal comparisons; also used in relational patterns (`case <= 0 => ...`). |
| `<<`, `>>` | `LEFTSHIFT`, `RIGHTSHIFT` | Bitwise | Bitwise left shift and arithmetic right shift on 64-bit integers. |
| `->>` | `JSON_GET_TEXT` | Database JSON | Postgres-style JSON extract as text operator. |
| `@>`, `<@` | `JSON_CONTAINS`, `JSON_CONTAINED` | Database JSON | Postgres-style JSONB containment operators. |
| `$@` | `DOLLAR AT` | Metaprogramming | Unquote-splicing prefix in macros (`$@list`). |
| `+=`, `-=`, `*=`, `/=`, `//=`, `%=`, `**=`, `&=`, `\|=`, `^=`, `<<=`, `>>=` | `*EQUAL` | Augmented Assignment | Evaluates binary operation on target and right-hand side and assigns back to target. |

### 10.2 Single-Character Operators & Delimiters

| Symbol | Token Name | All Syntactic Roles in Pilot |
| :--- | :--- | :--- |
| `+` | `PLUS` | Numeric addition, string concatenation (if either operand is `str`), list concatenation (`[1] + [2]`), matrix addition, unary plus (`+x`). |
| `-` | `MINUS` | Numeric subtraction, matrix subtraction, unary negation (`-x`), **Algebraic Class Subtraction** (`ClassA - ClassB` or `ClassA - "memberName"`). |
| `*` | `STAR` | Numeric multiplication, matrix multiplication (`Matrix * Matrix`), matrix scalar scaling (`Matrix * scalar`), string repetition (`"ab" * 3`), **Multi-Cast Method Group Broadcast** (`fn*(arg)`), **Repeated Call Unrolling** (`append*(list)`), positional varargs (`*args`), collection spread (`[*items]`), SQL `SELECT *`, unit multiplication (`kg * m`). |
| `/` | `SLASH` | Numeric division (throws `PilotDivideByZeroException` on zero), unit division (`m / s`). |
| `%` | `PERCENT` | Numeric modulo / remainder (throws `PilotDivideByZeroException` on zero). |
| `\|` | `VBAR` | Bitwise OR on integers, logical OR on booleans, **Type Union** (`int \| str`), **Pattern OR** (`case 1 \| 2`), **Method Group Union/Fallback** (`Json.serialize \| Xml.serialize`), **Algebraic Class Union** (`BaseUser \| AdminPrivileges`). |
| `&` | `AMPER` | Bitwise AND on integers, logical AND on booleans, **Type Intersection** (`Readable & Writable`), **Algebraic Class Intersection** (`ClassA & ClassB`), lambda reference capture (`[&x]`). |
| `^` | `CIRCUMFLEX` | Bitwise XOR on integers. |
| `~` | `TILDE` | Bitwise NOT (complement). |
| `<`, `>` | `LESS`, `GREATER` | Less-than / greater-than comparisons, generic type arguments (`List<int>`), dimensional unit annotations (`100 <m/s>`), relational patterns (`case > 100`). |
| `=` | `EQUAL` | Variable/field assignment, default parameter value, named argument (`color="Red"`), algebraic class definition (`class A = B \| C`), lambda value capture (`[=]`). |
| `?` | `QUESTION` | Nullable type suffix (`int?`). |
| `!` | `EXCLAMATION` | Postfix non-null assertion (`val!`), macro definition/invocation marker (`macro!`, `log!(x)`), preprocessor logical NOT (`#if !DEBUG`). |
| `@` | `AT` | Decorator prefix (`@vectorized`, `@hard_realtime`, `@wcet(50us)`), database index hint (`@index(id)`). |
| `$` | `DOLLAR` | Macro splice / unquote prefix (`$expr`). |
| `` ` `` | `BACKQUOTE` | Quasiquote prefix (`` `{ x + $y } ``). |
| `_` | `UNDERSCORE` | Wildcard discard pattern in `match` arms and destructuring assignments (`case _ => ...`). |
| `(` `)` | `LPAR`, `RPAR` | Tuple literals `(1, 2)`, grouping, parameter lists, function calls, class base lists. |
| `[` `]` | `LSQB`, `RSQB` | List literals `[1, 2]`, indexing `a[i]`, slicing `a[1..3]`, generic type parameters `Box[T]`, lambda capture lists `[x, y]`, dimensional units `100 [m]`, selective method stealing `steal from A [m1, m2]`. |
| `{` `}` | `LBRACE`, `RBRACE` | Code suites/blocks `{ ... }`, map literals `{"k": v}`, set literals `{1, 2, 3}`, structural record patterns `{ "id": id }`, projection targets `obj -> { name }`, f-string interpolation `{expr}`, inline polyglot blocks `js { ... }`. |
| `:` | `COLON` | Type annotation (`x: int`), suite introducer (`if cond:`), map key-value separator (`"k": v`), symbol literal (`:sym`), foreign target separator (`python:math`), labeled statement (`outer: for ...`), `where` constraint (`where T: IFoo`). |
| `;` | `SEMI` | Statement separator/terminator, fixed-size array type separator (`[u8; 64]`). Note: Semicolons at the end of lines are optional in Pilot (`SEMI?`). |
| `,` | `COMMA` | Element, argument, parameter, and pattern separator (trailing commas are permitted in all lists, tuples, maps, parameters, and patterns). |
| `.` | `DOT` | Member access (`obj.field`), dotted module paths (`a.b.c`), relative imports (`from ..pkg import x`), float decimal point. |

---

[**← Documentation Hub**](index.md) | [**Next: Syntax Reference →**](syntax-reference.md)
