# Pilot Available Functions, Methods & Runtime API Reference

[**← Syntax Reference**](syntax-reference.md) | [**Documentation Hub**](index.md) | **Functions & API Reference** | [**Concept 01 →**](concepts/01-types-variables-control-flow.md)

---

This reference catalogs **every available function, built-in module, scripting global, root object method, type member method/property, contract interface, type adapter, and runtime exception** in the Pilot language.

## Table of Contents

1. [Global Built-in Functions (`PilotBuiltinRegistry`)](#1-global-built-in-functions-pilotbuiltinregistry)
2. [Built-in Runtime Modules (`json`, `venv`, `TypeAdapter`)](#2-built-in-runtime-modules-json-venv-typeadapter)
3. [Scripting Host Globals & Functions (`PilotScriptContext`)](#3-scripting-host-globals--functions-pilotscriptcontext)
4. [Universal Root Object Methods (`PilotValue` & `PilotObject`)](#4-universal-root-object-methods-pilotvalue--pilotobject)
5. [Built-in Type Member Methods & Properties](#5-built-in-type-member-methods--properties)
   - [5.1 `str` (String)](#51-str-string)
   - [5.2 `list` (Dynamic Array / List)](#52-list-dynamic-array--list)
   - [5.3 `map` (Dictionary / Hash Map)](#53-map-dictionary--hash-map)
   - [5.4 `set` (Unique Hash Set)](#54-set-unique-hash-set)
   - [5.5 `matrix` (`PilotMatrix`)](#55-matrix-pilotmatrix)
   - [5.6 `range` (`PilotRange`)](#56-range-pilotrange)
   - [5.7 `MethodGroup` (`PilotMethodGroup`)](#57-methodgroup-pilotmethodgroup)
   - [5.8 `Generator` (`PilotGenerator`)](#58-generator-pilotgenerator)
   - [5.9 `Lazy` (`PilotLazy`)](#59-lazy-pilotlazy)
   - [5.10 `Future` (`PilotFuture`)](#510-future-pilotfuture)
   - [5.11 `Object` (`PilotObject` Class Instances)](#511-object-pilotobject-class-instances)
   - [5.12 `ClassNode` (`PilotClassNode` Generative AST Reflection)](#512-classnode-pilotclassnode-generative-ast-reflection)
6. [Design-by-Contract Intrinsics (`old` & `result`)](#6-design-by-contract-intrinsics-old--result)
7. [Standard Contracts & Delegated Interfaces (`PilotContractRegistry`)](#7-standard-contracts--delegated-interfaces-pilotcontractregistry)
8. [Built-in Type Adapters (`PilotTypeAdapterRegistry`)](#8-built-in-type-adapters-pilottypeadapterregistry)
9. [Polyglot Provider Built-in Functions (`PolyglotRegistry`)](#9-polyglot-provider-built-in-functions-polyglotregistry)
10. [Runtime Exception Hierarchy (`PilotExceptions`)](#10-runtime-exception-hierarchy-pilotexceptions)

---

## 1. Global Built-in Functions (`PilotBuiltinRegistry`)

Registered in [`Runtime/Builtins/PilotBuiltinRegistry.cs`](../Runtime/Builtins/PilotBuiltinRegistry.cs) and available in the global scope of every Pilot execution context and REPL session.

### 1.1 Standard Output & Introspection

| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`print(text...)`** | `null` | Converts all arguments to strings, joins them with a single space `" "`, and writes the result to `StandardOutput` **without** a trailing newline. |
| **`println(text...)`** | `null` | Converts all arguments to strings, joins them with `" "`, and writes a line to `StandardOutput` **with** a trailing newline. |
| **`len(collection)`** | `int` | Returns the element count of a `str` (character length), `list`, `tuple`, `map`, `set`, `range`, or `matrix` (`Rows * Cols`). Returns `0` if called with no arguments or a non-collection value. |
| **`type_of(value)`** | `str` | Returns the lowercase runtime kind name of `value` (`"int"`, `"uint"`, `"float"`, `"decimal"`, `"char"`, `"string"`, `"bool"`, `"list"`, `"tuple"`, `"map"`, `"set"`, `"range"`, `"matrix"`, `"function"`, `"methodgroup"`, `"object"`, `"generator"`, `"lazy"`, `"future"`, `"foreign"`, `"null"`, or `"undefined"`). |
| **`assert(condition, message?)`** | `bool` (`true`) | Evaluates `condition.IsTrue`. If `true`, returns `true`. If `false` (or no arguments provided), throws `PilotRuntimeException` with `message` (defaults to `"Assertion failed."`). Note: Pilot also supports the statement form `assert cond, "msg"`. |
| **`clock()`** | `float` | Returns a high-resolution monotonic timestamp in **milliseconds** (`Stopwatch.GetTimestamp() / Stopwatch.Frequency * 1000.0`). |

### 1.2 Type Casting & Coercion Functions

| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`str(val?)`** | `str` | Converts `val` to its string representation via `PilotValue.ToString()`. Returns `""` if called with no arguments. |
| **`int(val?)`** | `int` | Coerces `val` to a 64-bit signed integer (`AsInt()`): truncates `float`/`decimal`, converts `bool` (`true` -> `1`, `false` -> `0`), returns character code for `char`, or parses integer strings (returning `0` if unparseable or omitted). |
| **`float(val?)`** | `float` | Coerces `val` to a 64-bit floating-point number (`AsFloat()`): converts `int`/`uint`/`decimal`/`bool`, or parses invariant-culture floating-point strings (returning `0.0` if unparseable or omitted). |
| **`bool(val?)`** | `bool` | Evaluates the truthiness (`IsTrue`) of `val`: `false` for `false`, `0`, `0.0`, `NaN`, `""`, empty collections (`[]`, `()`, `{}`, empty `range`), `null`, and `undefined`; `true` for all other values. |

### 1.3 Ranges & Matrix Constructors

| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`range(end)`**<br>**`range(start, end)`**<br>**`range(start, end, step)`** | `range` (`PilotRange`) | Constructs a lazy integer sequence:<br>- 1 arg: `0` up to `end` (exclusive), step `1`.<br>- 2 args: `start` up to `end` (exclusive), step `1`.<br>- 3 args: `start` up to `end` (exclusive) incremented by `step`. |
| **`matrix(rows?, cols?)`** | `matrix` (`PilotMatrix`) | Creates a zero-initialized `float` matrix of dimensions `rows x cols` (`rows` defaults to `1`, `cols` defaults to `rows`). |
| **`eye(size?)`** | `matrix` (`PilotMatrix`) | Creates a `size x size` identity matrix with `1.0` on the main diagonal and `0.0` elsewhere (`size` defaults to `3`). |
| **`zeros(rows?, cols?)`** | `matrix` (`PilotMatrix`) | Creates a `rows x cols` matrix filled with `0.0` (`rows` defaults to `3`, `cols` defaults to `rows`). |
| **`ones(rows?, cols?)`** | `matrix` (`PilotMatrix`) | Creates a `rows x cols` matrix filled with `1.0` (`rows` defaults to `3`, `cols` defaults to `rows`). |

### 1.4 Homoiconic Lisp & Metaprogramming Functions

| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`gensym(prefix?)`** | `str` | Generates a globally unique, thread-safe symbol identifier string formatted as `"{prefix}__{id}"` (`prefix` defaults to `"sym"`, counter starts at `1001`). |
| **`cons(head, tail)`** | `list` | Prepends `head` to `tail`. If `tail` is a `list`, returns a new list `[head, ..tail]`. If `tail` is an atom, returns a two-element list `[head, tail]`. Returns `null` if fewer than 2 arguments are given. |
| **`car(list)`** | `any` | Returns the first element (`list[0]`) of a `list` (`null` if the list is empty). If passed a non-list value, returns that value directly. |
| **`cdr(list)`** | `list` | Returns a new `list` containing all elements of `list` after the first (`list.Skip(1)`). Returns `[]` if `list` is empty or not a list. |
| **`atom(val?)`** | `bool` | Returns `true` if `val` is an atomic value (any value whose `Kind` is neither `List` nor `Tuple`), or `false` if `val` is a `list` or `tuple`. |
| **`symbol(name?)`** | `str` | Returns a symbol representation for `name` (defaults to `""`). |
| **`eval(code)`** | `any` | Dynamically parses and executes the Pilot source string `code` using the current `PilotEngine` and `PilotExecutionOptions`, returning the resulting `PilotValue`. |

### 1.5 Coroutines, Concurrency & Lazy Evaluation

| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`next(generator)`** | `any` | Advances a `PilotGenerator` coroutine to its next `yield` point and returns the yielded value (or `undefined` when completed). Throws `PilotRuntimeException` if the argument is not a generator. |
| **`spawn(callable, args...)`** | `future` (`PilotFuture`) | Schedules `callable(args...)` to execute asynchronously on the .NET thread pool and immediately returns a `PilotFuture` handle. (Also available as a prefix operator `spawn expr` or block `spawn { ... }`). |
| **`parallel_map(callable, list)`** | `list` | Executes `callable(item)` concurrently across all elements of `list` using `Parallel.For`, preserving element order in the returned `list`. Throws `PilotRuntimeException` if arguments are not `(Function, List)`. |
| **`lazy(callable)`** | `lazy` (`PilotLazy`) | Wraps a zero-argument `callable` (or value) in a thread-safe `PilotLazy` thunk. The function is not executed until `.value` or `.eval()` is accessed, after which the result is memoized. |

### 1.6 JSON & Type Adaptation Built-ins

| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`json_dumps(value, formatted?)`** | `str` | Serializes any `PilotValue` (`map`, `list`, `object`, primitives) into a JSON string. If `formatted` is `true`, outputs indented pretty-printed JSON. |
| **`json_loads(json_string, target_class?)`** | `any` | Parses a JSON string into native Pilot values (`map`, `list`, `str`, `int`, `float`, `bool`, `null`). If `target_class` (a class constructor or class name string) is provided, hydrates JSON objects directly into instances of `target_class`. |
| **`register_adapter(from_type, to_type, converter)`** | `bool` | Programmatically registers a `PilotFunction` converter (`from_type -> to_type`) in `PilotTypeAdapterRegistry.Default`. Returns `true` on success. |
| **`vectorize(target)`** | `function` | Marks a `PilotFunction` with `IsVectorized = true` and returns it, enabling automatic collection unrolling on all subsequent calls to `target`. |
| **`auto_repeat()` / `repeat()`** | `str` | Sentinel tokens used with `with auto_repeat { ... }` / `with repeat { ... }` to enable scoped auto-vectorization. |

---

## 2. Built-in Runtime Modules (`json`, `venv`, `TypeAdapter`)

These built-in objects are pre-registered in `PilotBuiltinRegistry` and can be used directly (or after `import venv` / `import json`):

### 2.1 `json` Module
| Member | Signature | Description |
| :--- | :--- | :--- |
| **`json.dumps`** | `json.dumps(value, formatted?: bool) -> str` | Serializes `value` to a JSON string (compact by default, indented if `formatted == true`). |
| **`json.loads`** | `json.loads(json_string: str, target_class?: Class) -> any` | Deserializes `json_string` into Pilot values or an instance of `target_class`. |

### 2.2 `venv` / `virtual_environment` Module
Backed by [`Runtime/Venv/PilotVirtualEnvironment.cs`](../Runtime/Venv/PilotVirtualEnvironment.cs). Both `venv` and `virtual_environment` resolve to the same module object:

| Member | Signature | Description |
| :--- | :--- | :--- |
| **`venv.create`** | `venv.create(path: str, prompt?: str) -> str` | Creates a complete virtual environment directory layout at `path` (`pilotvenv.cfg`, `Scripts/` or `bin/` activation scripts, `lib/.pilot_modules/`, `include/`) and returns `path`. |
| **`venv.is_active`** | `venv.is_active() -> bool` | Returns `true` if a Pilot virtual environment is currently active in the process (`PilotVirtualEnvironment.IsActive`). |
| **`venv.active_path`** | `venv.active_path() -> str \| null` | Returns the absolute root directory path of the currently active virtual environment, or `null` if none is active. |
| **`venv.activate`** | `venv.activate(path: str) -> bool` | Activates the virtual environment at `path` in-process, redirecting `pilot pkg` module resolution to `<path>/lib/.pilot_modules`. Returns `true`. |
| **`venv.deactivate`** | `venv.deactivate() -> bool` | Deactivates the current virtual environment in-process and restores default module resolution. Returns `true`. |

### 2.3 `TypeAdapter` Module
| Member | Signature | Description |
| :--- | :--- | :--- |
| **`TypeAdapter.register`** | `TypeAdapter.register(from_type: str, to_type: str, converter: fn) -> bool` | Registers a custom type conversion function from `from_type` to `to_type` in `PilotTypeAdapterRegistry.Default`. |

---

## 3. Scripting Host Globals & Functions (`PilotScriptContext`)

When running scripts via `pilot <file.pilot>`, `pilot run`, `pilot -c`, or `PilotScript.Run()`, [`Scripting/PilotScriptContext.cs`](../Scripting/PilotScriptContext.cs) injects the following OS/process globals and file system utilities into the global scope:

### 3.1 Injected Global Variables
| Global Name | Type | Description |
| :--- | :--- | :--- |
| **`args`** | `list<str>` | List of command-line arguments passed to the script. |
| **`argv`** | `list<str>` | Alias for `args`. |
| **`script_path`** | `str` | Path to the executing `.pilot` or `.pilc` script file (`""` when executing inline `-c` code). |
| **`env`** | `map<str, str>` | Dictionary of all OS environment variables (`System.Environment.GetEnvironmentVariables()`), e.g., `env["PATH"]`. |

### 3.2 Process & File System Functions
| Function Signature | Return Type | Exact Behavior |
| :--- | :--- | :--- |
| **`cwd()`** | `str` | Returns the current working directory (`Directory.GetCurrentDirectory()`). |
| **`exit(code?: int)`** | *never returns* | Immediately terminates script execution with exit code `code` (defaults to `0`) by throwing `PilotScriptExitException`. |
| **`read_text(path: str)`** | `str` | Reads and returns the full UTF-8 text contents of the file at `path`. Returns `""` if the file does not exist. |
| **`write_text(path: str, content: str)`** | `bool` | Writes `content` to `path` (creating or overwriting the file) and returns `true` (`false` if fewer than 2 arguments are passed). |
| **`file_exists(path: str)`** | `bool` | Returns `true` if a file exists at `path`, otherwise `false`. |
| **`delete_file(path: str)`** | `bool` | Deletes the file at `path` if it exists and returns `true`; returns `false` if the file did not exist. |

---

## 4. Universal Root Object Methods (`PilotValue` & `PilotObject`)

Every value in Pilot—whether a primitive (`int`, `float`, `bool`, `str`), collection (`list`, `map`, `set`), callable, or user-defined class instance (`PilotObject`)—inherits the **Universal Root Object Protocol** (defined in `PilotValue.ResolveRootMember` and `PilotObject.GetMember`).

To support Python, C#, and Rust idioms seamlessly, each root method recognizes snake_case, PascalCase, and dunder aliases:

| Canonical Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`val.to_string()`** | `val.ToString()`, `val.__str__()` | `str` | Returns the string representation of the value (or invokes a custom `to_string`/`ToString`/`__str__` method if overridden on a user class). |
| **`val.type_of()`** | `val.GetType()` | `str` | For primitives/collections, returns the lowercase `PilotValueKind` name (`"int"`, `"list"`, etc.). For `PilotObject` instances, returns the class `TypeName` (e.g., `"SystemCoordinate"`), or invokes a custom `type_of` override. |
| **`val.hash()`** | `val.GetHashCode()`, `val.__hash__()` | `int` | Returns the 32/64-bit structural hash code of the value or object. |
| **`val.clone()`** | `val.Clone()`, `val.__clone__()` | `same type` | Performs a shallow copy of mutable collections (`list`, `map`, `set`) and `PilotObject` instances (copying all fields), or returns value types as-is. |
| **`val.equals(other)`** | `val.Equals(other)`, `val.__eq__(other)` | `bool` | Performs structural equality comparison (or invokes a custom `equals` override or `IEquatable` delegated contract on a class). |
| **`val.to_json(formatted?)`** | `val.toJson(formatted?)` | `str` | Serializes the value or object instance directly to a JSON string via `PilotJsonSerializer.Serialize`. |

---

## 5. Built-in Type Member Methods & Properties

### 5.1 `str` (String)
Defined in [`Runtime/Values/PilotValue.cs`](../Runtime/Values/PilotValue.cs) (lines 553–570):
- **Indexing**: `s[i]` returns the `char` at index `i` (supports negative indices `s[-1]` counting from the end; returns `undefined` if out of bounds).
- **Operators**: `s1 + s2` (concatenation), `s * n` (repeats string `n` times), `sub in s` (substring check).

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`s.length`** | *(property)* | `int` | Number of UTF-16 characters in `s`. |
| **`s.to_upper()`** | `s.ToUpper()` | `str` | Returns a copy of `s` converted to uppercase (`ToUpperInvariant()`). |
| **`s.to_lower()`** | `s.ToLower()` | `str` | Returns a copy of `s` converted to lowercase (`ToLowerInvariant()`). |
| **`s.trim()`** | `s.Trim()` | `str` | Removes leading and trailing whitespace from `s`. |
| **`s.starts_with(prefix)`** | `s.StartsWith(prefix)` | `bool` | Returns `true` if `s` begins with `prefix.ToString()`. |
| **`s.ends_with(suffix)`** | `s.EndsWith(suffix)` | `bool` | Returns `true` if `s` ends with `suffix.ToString()`. |
| **`s.contains(substr)`** | `s.Contains(substr)` | `bool` | Returns `true` if `substr.ToString()` occurs within `s`. |

### 5.2 `list` (Dynamic Array / List)
Defined in [`Runtime/Values/PilotValue.cs`](../Runtime/Values/PilotValue.cs) (lines 461–504, 583–627):
- **Indexing**: `list[i]` reads or writes element at `i` (supports negative indexing `list[-1]`).
- **Operators**: `list1 + list2` (returns a new concatenated list), `x in list` (membership check), `[..list1, 4]` (spread).

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`list.length`** | `list.count` *(property)* | `int` | Number of elements currently in the list. |
| **`list.append(item)`** | `list.push(item)` | `null` | Appends `item` to the end of the list in place. If `with auto_repeat` is active and `item` is a collection (`list`/`tuple`/`set`), unrolls and appends each element individually. |
| **`list.append.each(items)`** | `list.append.repeat(items)`, `list.append.broadcast(items)` | `null` | Sub-methods on `append`/`push` that unroll a collection `items` and append each element to `list`. |
| **`list.map(callable)`** | — | `list` | Applies `callable` (either a `PilotFunction` or a **`PilotMethodGroup` switchboard**) to each element in `list` and returns a new `list` of results. |
| **`list.pop()`** | — | `any` | Removes and returns the last element of `list` (or `undefined` if empty). |
| **`list.clear()`** | — | `null` | Removes all elements from `list` in place. |

> **Note**: When a class delegates `IList` to a backing `list` field (`IList.Contracts = _data`), the delegated contract also exposes `.add(x)`, `.remove(x)`, `.pop(idx?)`, `.count()`, `.length()`, `.get(idx)`, `.set(idx, val)`, `.clear()`, `.contains(x)`, and `.insert(idx, val)`.

### 5.3 `map` (Dictionary / Hash Map)
Defined in [`Runtime/Values/PilotValue.cs`](../Runtime/Values/PilotValue.cs) (lines 506–529):
- **Bracket Indexing**: `map[key]` reads or sets the value associated with `key` (returns `undefined` if not found).
- **Dot Property Access**: `map.key_name` first checks if `"key_name"` exists as a string key in `map` and returns its value before falling back to map methods.
- **Operators**: `key in map` (checks if `key` exists in `map`), `{ ..map1, "k": v }` (spread).

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`map.length`** | `map.count` *(property)* | `int` | Number of key-value pairs in the map. |
| **`map.keys()`** | — | `list` | Returns a new `list` containing all keys in the map. |
| **`map.values()`** | — | `list` | Returns a new `list` containing all values in the map. |
| **`map.contains_key(key)`** | `map.ContainsKey(key)` | `bool` | Returns `true` if `key` exists in the map. |
| **`map.clear()`** | — | `null` | Removes all key-value entries from the map in place. |

### 5.4 `set` (Unique Hash Set)
Defined in [`Runtime/Values/PilotValue.cs`](../Runtime/Values/PilotValue.cs) (lines 531–551):
- **Literal Syntax**: `{ 1, 2, 3 }` (braces with elements without `:` colons create a `set`; `{}` creates an empty `map`).

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`set.length`** | `set.count` *(property)* | `int` | Number of unique elements in the set. |
| **`set.add(item)`** | — | `null` | Adds `item` to the set if not already present. |
| **`set.contains(item)`** | `set.Contains(item)` | `bool` | Returns `true` if `item` is a member of the set. |
| **`set.clear()`** | — | `null` | Removes all elements from the set in place. |

### 5.5 `matrix` (`PilotMatrix`)
Defined in [`Runtime/Values/PilotMatrix.cs`](../Runtime/Values/PilotMatrix.cs):
- Constructed via `matrix(rows, cols)`, `eye(size)`, `zeros(rows, cols)`, `ones(rows, cols)`.
- **Supported Operators**:
  - `A + B`: Element-wise matrix addition (throws if dimensions differ).
  - `A - B`: Element-wise matrix subtraction (throws if dimensions differ).
  - `A * B`: Matrix-matrix dot product ($m \times k$ multiplied by $k \times n$ yields $m \times n$).
  - `A * scalar`: Multiplies every element of matrix `A` by numeric `scalar`.
  - `A == B`: Structural equality comparing dimensions and elements within `1e-9` tolerance.
  - `len(A)`: Returns `Rows * Cols`.

### 5.6 `range` (`PilotRange`)
- Constructed via `start..end` (half-open), `start..=end` (inclusive), or `range(start, end, step)`.
- `len(r)` returns the total number of integers in the range.
- Directly iterable in `for i in 0..10` and unpackable via spread `[..0..5]` -> `[0, 1, 2, 3, 4]`.

### 5.7 `MethodGroup` (`PilotMethodGroup`)
Defined in [`Runtime/Values/PilotMethodGroup.cs`](../Runtime/Values/PilotMethodGroup.cs) and [`PilotValue.cs`](../Runtime/Values/PilotValue.cs) (lines 445–458):

| Member / Operator | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`mg.name`** | `str` *(property)* | Name of the method group (or `"A\|B"` for union groups). |
| **`mg.overloads`** | `list<OverloadInfo>` *(property)* | Returns a list of reflection objects, one per overload in the group. Each `OverloadInfo` has:<br>- `.name` (`str`)<br>- `.signature` (`str`, e.g., `"(x: int) -> any"`)<br>- `.return_type` (`str`)<br>- `.is_async` (`bool`)<br>- `.is_generator` (`bool`)<br>- `.parameters` / `.args` (`list<map>` with `"name"`, `"type"`, `"is_params"`). |
| **`mg.broadcast(args...)`** | `fn -> list` | Invokes **all** matching overloads in the group with `args` and returns a `list` of their return values (identical to `mg*(args...)`). |
| **`mg1 \| mg2`** | `MethodGroup` | Merges two method groups or functions into a unified fallback overload set. |
| **`mg(partial_args)`** | `MethodGroup` | **Signature Sieve Currying**: When called with partial positional or named arguments that do not completely satisfy an overload, filters the group down to compatible overloads with those arguments pre-bound. |

### 5.8 `Generator` (`PilotGenerator`)
Defined in [`Runtime/Values/PilotGenerator.cs`](../Runtime/Values/PilotGenerator.cs):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`gen.next()`** | `fn() -> any` | Resumes the generator with `undefined` input and returns the next yielded value (or `undefined` if completed). |
| **`gen.send(val)`** | `fn(any) -> any` | Sends `val` into the suspended `yield` expression, resumes execution to the next `yield`, and returns the yielded value. |
| **`gen.close()`** | `fn() -> null` | Transitions generator state to `Closed`, cancels any active runner thread, and invokes cleanup callbacks. |
| **`gen.is_done`** | `bool` *(property)* | `true` if `State` is `Completed` or `Closed`, otherwise `false`. |
| **`gen.state`** | `str` *(property)* | Current lifecycle state string: `"suspended"`, `"running"`, `"completed"`, or `"closed"`. |

### 5.9 `Lazy` (`PilotLazy`)
Defined in [`Runtime/Values/PilotLazy.cs`](../Runtime/Values/PilotLazy.cs):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`l.value`** | `any` *(property)* | Evaluates the underlying thunk on first access (thread-safe via lock), caches the result, and returns it on all subsequent accesses. |
| **`l.eval()`** | `fn() -> any` | Method form of `.value`; forces evaluation (if not yet evaluated) and returns the cached value. |
| **`l.is_evaluated`** | `bool` *(property)* | `true` if the lazy thunk has already been evaluated and cached, `false` otherwise. |
| **`l.<any_member>`** | `any` | Accessing any other member on a `PilotLazy` automatically forces evaluation and forwards the member lookup to the inner `Value`. |

### 5.10 `Future` (`PilotFuture`)
Defined in [`Runtime/Concurrency/PilotFuture.cs`](../Runtime/Concurrency/PilotFuture.cs):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`f.await()`** | `fn() -> any` | Blocks until the background task completes and returns its `PilotValue` result (identical to `await f`). |
| **`f.get_result(timeout_ms?)`** | `fn(int?) -> any` (alias `f.get`) | Waits up to `timeout_ms` milliseconds for the task to complete (waits indefinitely if omitted or `< 0`). Throws `TimeoutException` if the timeout expires. |
| **`f.cancel()`** | `fn() -> null` | Requests cancellation on the task's `CancellationTokenSource`. |
| **`f.is_completed`** | `bool` *(property)* | `true` if the task has finished execution. |
| **`f.is_faulted`** | `bool` *(property)* | `true` if the task terminated with an unhandled exception. |

### 5.11 `Object` (`PilotObject` Class Instances)
Defined in [`Runtime/Values/PilotObject.cs`](../Runtime/Values/PilotObject.cs):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`obj.current_state()`** | `fn() -> str \| null` (alias `obj.currentState()`) | Returns the name of the active typestate (e.g., `"Disconnected"`, `"Connected"`), or `null` if the class does not use typestates. |
| **`obj.transition_to(newState)`** | `fn(str) -> Object` | Transitions the instance's `CurrentState` to `newState` and returns `self`, unlocking methods in the target `state` block and locking out methods exclusive to previous states. |
| **`__init__(self, ...)`** | Constructor | Automatically invoked when instantiating `ClassName(...)` (also supports `constructor(self, ...)` or a method matching `ClassName`). |

### 5.12 `ClassNode` (`PilotClassNode` Generative AST Reflection)
Defined in [`Runtime/Generative/PilotClassNode.cs`](../Runtime/Generative/PilotClassNode.cs). Passed as the `ast` argument to macros defined inside a class body:

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`ast.name`** | `str` *(property)* | Name of the class being compiled/constructed. |
| **`ast.fields`** | `list<FieldNode>` *(property)* | List of field metadata objects on the class. Each field object exposes `.name` (`str`), `.type` (`str`), and `.default_value` (`any`). |
| **`ast.add_field(name, default_value?)`** | `fn(str, any?) -> null` | Dynamically injects a new field into the class blueprint with an optional default value. |
| **`ast.add_method(name, body)`** | `fn(str, str) -> null` | Dynamically synthesizes and injects a new method (such as `"return self.<field>"` getters) onto the class blueprint. |

---

## 6. Design-by-Contract Intrinsics (`old` & `result`)

Inside `ensures` postcondition clauses on functions (`PilotEngine.InvokeFunction` and `PilotInterpreterVisitor.VisitAtom_expr`):

| Intrinsic | Context | Exact Behavior |
| :--- | :--- | :--- |
| **`old(expr)`** | `ensures` clause | Before the function body executes, `PilotEngine.SnapshotOldCalls` scans all `ensures` AST nodes for `old(...)` calls, evaluates `expr` in the pre-execution call scope, clones the value (`CloneValue()`), and stores it in `functionContext.OldSnapshots`. When `ensures` runs after the body completes, `old(expr)` returns that pre-state snapshot. |
| **`result`** | `ensures` clause | Automatically bound in the function's call scope to the return value produced by the function body before `ensures` clauses are evaluated (e.g., `ensures result > 0`). |

---

## 7. Standard Contracts & Delegated Interfaces (`PilotContractRegistry`)

Defined in [`Runtime/Contracts/PilotContractRegistry.cs`](../Runtime/Contracts/PilotContractRegistry.cs). When a class delegates a contract to a field or substitute (`IList.Contracts = _items`, `contracts for IList = _items`, or `delegate IList to _items`), Pilot automatically synthesizes forwarding methods for all methods in that contract not explicitly overridden on the class:

| Contract Name | Synthesized Forwarding Methods | Special Behavior |
| :--- | :--- | :--- |
| **`IList`** | `add`, `append`, `remove`, `pop`, `count`, `length`, `get`, `set`, `clear`, `contains`, `insert` | Directly dispatches to native `PilotValueKind.List` operations (`DispatchListMethod`) or custom object methods. |
| **`IEquatable`** | `equals` | Compares the delegated key/member on `self` against the corresponding delegated key/member on `other` (`DispatchEquals`). |
| **`IComparable`** | `compare_to`, `compareTo` | Forwards comparison calls to the delegated target member. |
| **`IMap` / `IDictionary`** | `get`, `set`, `contains_key`, `remove`, `keys`, `values`, `count`, `clear` | Directly dispatches to native `PilotValueKind.Map` operations (`DispatchMapMethod`) or custom object methods. |
| **`ISet`** | `add`, `remove`, `contains`, `count`, `clear` | Forwards set operations to the delegated target member. |
| **`IDisposable`** | `dispose` | Forwards resource cleanup to the delegated target member. |
| **`IEnumerable`** | `get_enumerator`, `__iter__` | Forwards iteration protocol calls to the delegated target member. |
| **Any User `interface` / `class`** | All method names defined on that interface/class in scope | Dynamically resolved from `context.Environment` and forwarded to the target member. |

---

## 8. Built-in Type Adapters (`PilotTypeAdapterRegistry`)

Defined in [`Runtime/Adapters/PilotTypeAdapterRegistry.cs`](../Runtime/Adapters/PilotTypeAdapterRegistry.cs). Whenever a function parameter has a type annotation `T` and receives a value of type `S`, Pilot automatically applies the matching adapter below (in addition to any user-defined `adapter S -> T` rules):

| Source Type (`From`) | Target Type (`To`) | Exact Conversion Rule |
| :--- | :--- | :--- |
| `str` / `string` | `int` | Trims string and parses invariant integer (or parses float and truncates to `long`). |
| `str` / `string` | `float` | Trims string and parses invariant `double`. |
| `str` | `bool` | Case-insensitive: `"true"`, `"1"`, `"yes"` -> `true`; `"false"`, `"0"`, `"no"` -> `false`. |
| `int` | `str` / `string` | Formats integer to invariant string. |
| `float` | `str` / `string` | Formats float to invariant string. |
| `bool` | `str` | `"true"` or `"false"`. |
| `int` | `float` | Widens 64-bit integer to `double`. |
| `float` | `int` | Truncates `double` to 64-bit `long`. |
| `list` | `tuple` | Wraps list elements in an immutable `PilotValueKind.Tuple`. |
| `tuple` | `list` | Copies tuple elements into a mutable `PilotValueKind.List`. |
| `list` | `set` | Copies list elements into a unique `HashSet<PilotValue>`. |
| `set` | `list` | Copies set elements into a `List<PilotValue>`. |
| `dict` / `map` | `object` | Instantiates an `AdaptedObject` and populates all map entries as object fields. |
| `map` | **Any User Class `C`** | **Structural Hydration**: Resolves class `C` in scope, instantiates `PilotObject(C)`, and copies every key-value pair from the `map` into the instance's fields. |

---

## 9. Polyglot Provider Built-in Functions (`PolyglotRegistry`)

Defined in [`Runtime/Interop/PolyglotRegistry.cs`](../Runtime/Interop/PolyglotRegistry.cs):
- **Python Provider (`DefaultPythonProvider`)**:
  - Evaluates inline `python { ... }` blocks, binding variables from the enclosing Pilot scope and supporting `yield` coroutines.
  - Direct member invocation (`Invoke`): `sin(x)`, `cos(x)`, `tan(x)`, `sqrt(x)`, `pi`, `e`, `dict()`, `list()`.
- **C# Provider (`DefaultCSharpProvider`)**:
  - Evaluates inline `csharp { ... }` blocks.
  - Direct member invocation (`Invoke`): `NewGuid()`, `Now`, `Sin(x)`, `Cos(x)`, `Abs(x)`.
- **JavaScript (`js`), Java (`java`), Lua (`lua`), C (`c`), C++ (`cpp`) Providers**:
  - Evaluate `return <expr>;` statements, variable lookups from Pilot scope, and standard output calls (`console.log(...)`, `System.out.println(...)`, `print(...)`, `printf(...)`, `std::cout << ...`).
- **Assembly Provider (`DefaultAsmProvider`)**:
  - Simulates 32/64-bit x86/x64 registers (`eax`, `ebx`, `ecx`, `edx`, `rax`, `rbx`, `rcx`, `rdx`), executing `mov dest, src`, `add dest, src` (reading immediate values, registers, or Pilot scope variables), and `ret`, returning the final value of `eax` / `rax`.
- **PostgreSQL (`postgresql`) & MySQL (`mysql`) Providers**:
  - Execute SQL `SELECT` queries (including `WHERE id IN (...)` filtering) and return a `list` of structured `PilotObject` rows with `.id`, `.name`, and `.count` fields, or return affected row counts (`1`) for DML commands.

---

## 10. Runtime Exception Hierarchy (`PilotExceptions`)

Defined in [`Runtime/ControlFlow/PilotExceptions.cs`](../Runtime/ControlFlow/PilotExceptions.cs). All runtime errors inherit from `PilotRuntimeException` and can be caught in `try ... except <TypeName> as err` blocks using either their short Pilot name or full C# class name:

| Pilot Exception Name | C# Class Name | Trigger Condition |
| :--- | :--- | :--- |
| **`Exception` / `Error` / `RuntimeError`** | `PilotRuntimeException` | Base class catching any runtime exception in Pilot. |
| **`TypeError`** | `PilotTypeException` | Type mismatch, no matching overload in a method group, calling a non-function, or calling a typestate method in the wrong state. |
| **`DivideByZeroError`** | `PilotDivideByZeroException` | Division (`/`, `//`) or modulo (`%`) by `0` or `0.0`. |
| **`IndexError`** | `PilotIndexOutOfRangeException` | Out-of-bounds index on a matrix or strict sequence. |
| **`KeyError`** | `PilotKeyNotFoundException` | Missing required key in dictionary/map operations. |
| **`NullReferenceError`** | `PilotNullReferenceException` | Accessing `.member` on `null` or `undefined` without safe navigation `?.`. |
| **`AssertionError`** | `PilotAssertionException` | `assert condition` statement evaluated to `false`. |
| **`RecursionError`** | `PilotRecursionException` | Call stack depth exceeded `PilotExecutionOptions.MaxCallDepth`. |
| **`ContractError`** | `PilotContractException` | Design-by-Contract `requires` precondition or `ensures` postcondition evaluated to `false`. |
| **`TimeoutError`** | `PilotTimeoutException` | Asynchronous future or concurrent operation exceeded its deadline/timeout. |
| **`MatchError`** | `PilotPatternMatchingException` | Destructuring assignment (`[a, b] = ...` or `{ "k": v } = ...`) failed to match the runtime structure or arity. |
| **`ContextError`** | `PilotContextException` | A required `context` parameter could not be resolved from any enclosing `with context` block. |
| **`UserError` (or custom class name)** | `PilotUserException` | Explicitly thrown by a user `raise expr` statement; preserves `UserPayload` when caught via `except as err`. |

---

[**← Syntax Reference**](syntax-reference.md) | [**Documentation Hub**](index.md) | [**Next: Concept 01 — Types, Variables & Control Flow →**](concepts/01-types-variables-control-flow.md)
