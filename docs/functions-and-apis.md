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
   - [5.3 `int` (64-Bit Integer)](#53-int-64-bit-integer)
   - [5.4 `float` (64-Bit Floating-Point)](#54-float-64-bit-floating-point)
   - [5.5 `bool` (Boolean)](#55-bool-boolean)
   - [5.6 `map` (Dictionary / Hash Map)](#56-map-dictionary--hash-map)
   - [5.7 `set` (Unique Hash Set)](#57-set-unique-hash-set)
   - [5.8 `matrix` (`PilotMatrix`)](#58-matrix-pilotmatrix)
   - [5.9 `range` (`PilotRange`)](#59-range-pilotrange)
   - [5.10 `MethodGroup` (`PilotMethodGroup`)](#510-methodgroup-pilotmethodgroup)
   - [5.11 `Generator` (`PilotGenerator`)](#511-generator-pilotgenerator)
   - [5.12 `Lazy` (`PilotLazy`)](#512-lazy-pilotlazy)
   - [5.13 `Future` (`PilotFuture`)](#513-future-pilotfuture)
   - [5.14 `Object` (`PilotObject` Class Instances)](#514-object-pilotobject-class-instances)
   - [5.15 `ClassNode` (`PilotClassNode` Generative AST Reflection)](#515-classnode-pilotclassnode-generative-ast-reflection)
6. [Design-by-Contract Intrinsics (`old` & `result`)](#6-design-by-contract-intrinsics-old--result)
7. [Standard Contracts & Delegated Interfaces (`PilotContractRegistry`)](#7-standard-contracts--delegated-interfaces-pilotcontractregistry)
8. [Built-in Type Adapters (`PilotTypeAdapterRegistry`)](#8-built-in-type-adapters-pilottypeadapterregistry)
9. [Polyglot Provider Built-in Functions (`PolyglotRegistry`)](#9-polyglot-provider-built-in-functions-polyglotregistry)
10. [Runtime Exception Hierarchy (`PilotExceptions`)](#10-runtime-exception-hierarchy-pilotexceptions)
11. [Standard Library Modules & Native Resolvers (`stdlib/`)](#11-standard-library-modules--native-resolvers-stdlib)
   - [11.1 Module Architecture & Provenance](#111-module-architecture--provenance)
   - [11.2 Core & Memory (`std.core`, `std.option`, `std.result`, `std.mem`, `std.meta`)](#112-core--memory)
   - [11.3 Collections & Iteration (`std.collections`, `std.itertools`, `std.text`)](#113-collections--iteration)
   - [11.4 Mathematics & Randomness (`std.math`, `std.random`, `std.decimal`)](#114-mathematics--randomness)
   - [11.5 I/O & File System (`std.io`, `std.fs`, `std.path`, `std.pathlib`, `std.shutil`)](#115-io--file-system)
   - [11.6 Operating System & Process (`std.os`, `std.sys`)](#116-operating-system--process)
   - [11.7 Functional Programming (`std.functools`)](#117-functional-programming)
   - [11.8 Serialization & Formats (`std.json`, `std.csv`, `std.pickle`, `std.tomllib`, `std.encoding`, `std.formats`)](#118-serialization--formats)
   - [11.9 Concurrency & Asynchronous (`std.threading`, `std.multiprocessing`, `std.asyncio`)](#119-concurrency--asynchronous)
   - [11.10 Networking & HTTP (`std.socket`, `std.urllib`, `std.http`, `std.request`)](#1110-networking--http)
   - [11.11 Data Science & Scientific Computing (`std.numpy`, `std.pandas`, `std.matplotlib`, `std.scikit`)](#1111-data-science--scientific-computing)
   - [11.12 Testing, Profiling & Security (`std.unittest`, `std.logging`, `std.timeit`, `std.secrets`, `std.crypto`, `std.time`)](#1112-testing-profiling--security)
   - [11.13 Module Resolver & Precompiled Bytecode Cache (`PilotStdlibResolver`)](#1113-module-resolver--precompiled-bytecode-cache-pilotstdlibresolver)
   - [11.14 Linkable C-ABI Native Runtime API (`libpilot_rt.a`, `libpilot_rt.so`, `pilot_runtime.h`)](#1114-linkable-c-abi-native-runtime-api)

---

## 1. Global Built-in Functions (`PilotBuiltinRegistry`)

Registered natively in the runtime intrinsics engine ([`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm)) and available in the global scope of every Pilot execution context and REPL session.

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
Backed by [`pilot-pkg`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-pkg) and [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt). Both `venv` and `virtual_environment` resolve to the same module object:

| Member | Signature | Description |
| :--- | :--- | :--- |
| **`venv.create`** | `venv.create(path: str, prompt?: str) -> str` | Creates a complete virtual environment directory layout at `path` (`pilot-venv.json`, `bin/` activation scripts, `.pilot_modules/`) and returns `path`. |
| **`venv.is_active`** | `venv.is_active() -> bool` | Returns `true` if a Pilot virtual environment is currently active in the process. |
| **`venv.active_path`** | `venv.active_path() -> str \| null` | Returns the absolute root directory path of the currently active virtual environment, or `null` if none is active. |
| **`venv.activate`** | `venv.activate(path: str) -> bool` | Activates the virtual environment at `path` in-process, redirecting `pilot pkg` module resolution to `<path>/.pilot_modules`. Returns `true`. |
| **`venv.deactivate`** | `venv.deactivate() -> bool` | Deactivates the current virtual environment in-process and restores default module resolution. Returns `true`. |

### 2.3 `TypeAdapter` Module
| Member | Signature | Description |
| :--- | :--- | :--- |
| **`TypeAdapter.register`** | `TypeAdapter.register(from_type: str, to_type: str, converter: fn) -> bool` | Registers a custom type conversion function from `from_type` to `to_type` in the type adapter registry (`pilot-analysis`). |

---

## 3. Scripting Host Globals & Functions (`PilotScriptContext`)

When running scripts via `pilot <file.pilot>`, `pilot run <file.pilot>`, or `pilot compile`, the runtime CLI driver ([`pilot-cli`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-cli)) and [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) inject the following OS/process globals and file system utilities into the global scope:

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
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) ([`c_abi.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/c_abi.rs), [`stdlib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/stdlib.rs)), [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis), [`pilot-codegen`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-codegen), and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):
- **Indexing**: `s[i]` returns the `char` / 1-character string at index `i` (supports negative indices `s[-1]` counting from the end; returns `undefined` or `""` if out of bounds).
- **Operators**: `s1 + s2` (concatenation), `s * n` (repeats string `n` times), `sub in s` (substring check).
- **Interpolation**: Native string concatenation and interpolation convert integers, floats, booleans, and lists automatically.

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`s.len()`** | `s.length` *(property)* | `int` | Number of characters in `s`. |
| **`s.is_empty()`** | — | `bool` | Returns `true` if `s` has length `0`, otherwise `false`. |
| **`s.contains(substr)`** | `s.Contains(substr)` | `bool` | Returns `true` if `substr` occurs within `s`. |
| **`s.starts_with(prefix)`** | `s.StartsWith(prefix)` | `bool` | Returns `true` if `s` begins with `prefix`. |
| **`s.ends_with(suffix)`** | `s.EndsWith(suffix)` | `bool` | Returns `true` if `s` ends with `suffix`. |
| **`s.index_of(substr)`** | `s.IndexOf(substr)` | `int` | Returns the first 0-based index of `substr` in `s`, or `-1` if not found. |
| **`s.last_index_of(substr)`** | `s.LastIndexOf(substr)` | `int` | Returns the last 0-based index of `substr` in `s`, or `-1` if not found. |
| **`s.to_upper()`** | `s.ToUpper()` | `str` | Returns a copy of `s` converted to uppercase. |
| **`s.to_lower()`** | `s.ToLower()` | `str` | Returns a copy of `s` converted to lowercase. |
| **`s.trim()`** | `s.Trim()` | `str` | Removes leading and trailing whitespace from `s`. |
| **`s.trim_start()`** | `s.TrimStart()` | `str` | Removes leading whitespace from `s`. |
| **`s.trim_end()`** | `s.TrimEnd()` | `str` | Removes trailing whitespace from `s`. |
| **`s.replace(old, new)`** | `s.Replace(old, new)` | `str` | Returns a new string with all occurrences of `old` replaced with `new`. |
| **`s.split(sep)`** | `s.Split(sep)` | `list<str>` | Splits `s` into a list of strings by delimiter `sep` (or by whitespace if `sep` is `" "`). |
| **`s.join(items)`** | `s.Join(items)` | `str` | Joins the elements of `items` (`list<str>`) using `s` as the separator delimiter. |
| **`s.substring(start, end?)`** | `s.slice(start, end)` | `str` | Returns a sub-string from index `start` up to `end` (or string end if omitted). Supports negative indices. |
| **`s.repeat(count)`** | — | `str` | Returns a new string containing `s` repeated `count` times. |
| **`s.pad_start(len, pad_char)`** | `s.PadLeft(len, pad_char)` | `str` | Prepends `pad_char` to `s` until the total length reaches `len`. |
| **`s.pad_end(len, pad_char)`** | `s.PadRight(len, pad_char)` | `str` | Appends `pad_char` to `s` until the total length reaches `len`. |
| **`s.is_digit()`** | `s.IsDigit()` | `bool` | Returns `true` if all characters in non-empty `s` are ASCII digits (`0`–`9`). |
| **`s.is_alpha()`** | `s.IsAlpha()` | `bool` | Returns `true` if all characters in non-empty `s` are alphabetic (`a`–`z`, `A`–`Z`). |
| **`s.is_alphanumeric()`** | `s.IsAlphanumeric()` | `bool` | Returns `true` if all characters in non-empty `s` are alphanumeric (`0`–`9`, `a`–`z`, `A`–`Z`). |
| **`s.char_at(index)`** | `s.CharAt(index)` | `str` | Returns the 1-character string at 0-based `index` (supports negative indices). |
| **`s.chars()`** | `s.Chars()` | `list<str>` | Returns a list containing each individual character of `s` as a 1-character string. |
| **`s.to_int()`** | `s.ToInt()` | `int` | Parses string to a 64-bit signed integer (supports decimal, `0x` hex, and `0b` binary). |
| **`s.to_float()`** | `s.ToFloat()` | `float` | Parses string to a 64-bit IEEE 754 floating-point number. |
| **`s.to_string()`** | `s.ToString()` | `str` | Returns a copy of `s`. |

### 5.2 `list` (Dynamic Array / List)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) ([`c_abi.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/c_abi.rs), [`stdlib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/stdlib.rs)), [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis), [`pilot-codegen`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-codegen), and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):
- **Indexing**: `list[i]` reads or writes element at `i` (supports negative indexing `list[-1]`).
- **Operators**: `list1 + list2` (returns a new concatenated list), `x in list` (membership check), `[..list1, 4]` (spread).

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`list.len()`** | `list.length`, `list.count` *(property)* | `int` | Number of elements currently in the list. |
| **`list.is_empty()`** | — | `bool` | Returns `true` if the list contains 0 elements, otherwise `false`. |
| **`list.append(item)`** | `list.push(item)`, `list.add(item)` | `null` | Appends `item` to the end of the list in place. |
| **`list.pop()`** | — | `T` / `any` | Removes and returns the last element of the list (or `undefined`/`null` if empty). |
| **`list.insert(index, item)`** | — | `null` | Inserts `item` at 0-based `index` in place, shifting existing elements right. |
| **`list.remove(index)`** | — | `T` / `any` | Removes and returns the element at 0-based `index`, shifting subsequent elements left. |
| **`list.clear()`** | — | `null` | Removes all elements from the list in place, resetting length to `0`. |
| **`list.reverse()`** | — | `list` | Reverses the elements of the list in place and returns the list instance. |
| **`list.contains(item)`** | `list.Contains(item)` | `bool` | Returns `true` if `item` equals any element in the list. |
| **`list.index_of(item)`** | `list.IndexOf(item)` | `int` | Returns the 0-based index of the first occurrence of `item`, or `-1` if not found. |
| **`list.slice(start, end?)`** | — | `list` | Returns a new sub-list containing elements from index `start` up to `end` (supports negative indices). |
| **`list.first()`** | — | `T` / `any` | Returns the first element of the list (or `undefined`/`null` if empty). |
| **`list.last()`** | — | `T` / `any` | Returns the last element of the list (or `undefined`/`null` if empty). |
| **`list.join(sep)`** | `list.Join(sep)` | `str` | Joins string elements of the list using `sep` as delimiter. |
| **`list.map(callable)`** | — | `list` | Applies `callable` (a function, lambda, or method group) to each element and returns a new `list`. |

### 5.3 `int` (64-Bit Integer)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt), [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis), and [`pilot-codegen`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-codegen):

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`n.abs()`** | `n.Abs()` | `int` | Returns the absolute value of `n` (`|n|`). |
| **`n.clamp(min, max)`** | `n.Clamp(min, max)` | `int` | Restricts `n` to the closed interval `[min, max]`. |
| **`n.min(other)`** | `n.Min(other)` | `int` | Returns the smaller of `n` and `other`. |
| **`n.max(other)`** | `n.Max(other)` | `int` | Returns the larger of `n` and `other`. |
| **`n.to_string()`** | `n.to_str()`, `n.ToString()` | `str` | Converts `n` to its base-10 decimal string representation. |
| **`n.to_float()`** | `n.ToFloat()` | `float` | Converts `n` to a 64-bit IEEE 754 floating-point number. |
| **`n.to_hex()`** | `n.ToHex()` | `str` | Converts `n` to a lowercase hexadecimal string with `0x` prefix (e.g. `0xff`). |
| **`n.to_bin()`** | `n.ToBin()` | `str` | Converts `n` to a binary string with `0b` prefix (e.g. `0b1010`). |
| **`n.bit_length()`** | `n.BitLength()` | `int` | Returns the number of bits necessary to represent `n` in binary (excluding sign). |

### 5.4 `float` (64-Bit Floating-Point)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt), [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis), and [`pilot-codegen`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-codegen):

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`f.abs()`** | `f.Abs()` | `float` | Returns the absolute value of `f` (`|f|`). |
| **`f.clamp(min, max)`** | `f.Clamp(min, max)` | `float` | Restricts `f` to the closed interval `[min, max]`. |
| **`f.min(other)`** | `f.Min(other)` | `float` | Returns the smaller of `f` and `other`. |
| **`f.max(other)`** | `f.Max(other)` | `float` | Returns the larger of `f` and `other`. |
| **`f.round()`** | `f.Round()` | `float` | Rounds `f` to the nearest integer value (half-way cases round away from zero). |
| **`f.floor()`** | `f.Floor()` | `float` | Rounds `f` downward toward negative infinity ($\lfloor f \rfloor$). |
| **`f.ceil()`** | `f.Ceil()` | `float` | Rounds `f` upward toward positive infinity ($\lceil f \rceil$). |
| **`f.trunc()`** | `f.Trunc()` | `float` | Truncates the fractional part of `f` toward zero. |
| **`f.sqrt()`** | `f.Sqrt()` | `float` | Computes the square root ($\sqrt{f}$). |
| **`f.pow(exp)`** | `f.Pow(exp)` | `float` | Computes $f^{\text{exp}}$. |
| **`f.to_string()`** | `f.to_str()`, `f.ToString()` | `str` | Converts `f` to its decimal string representation. |
| **`f.to_int()`** | `f.ToInt()` | `int` | Truncates `f` toward zero and casts to a 64-bit signed integer. |

### 5.5 `bool` (Boolean)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt), [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis), and [`pilot-codegen`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-codegen):

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`b.to_string()`** | `b.to_str()`, `b.ToString()` | `str` | Returns `"true"` or `"false"`. |
| **`b.toggle()`** | `b.Toggle()` | `bool` | Returns the logical negation (`!b`). |

### 5.6 `map` (Dictionary / Hash Map)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):
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

### 5.7 `set` (Unique Hash Set)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):
- **Literal Syntax**: `{ 1, 2, 3 }` (braces with elements without `:` colons create a `set`; `{}` creates an empty `map`).

| Member / Method | Aliases | Return Type | Exact Behavior |
| :--- | :--- | :--- | :--- |
| **`set.length`** | `set.count` *(property)* | `int` | Number of unique elements in the set. |
| **`set.add(item)`** | — | `null` | Adds `item` to the set if not already present. |
| **`set.contains(item)`** | `set.Contains(item)` | `bool` | Returns `true` if `item` is a member of the set. |
| **`set.clear()`** | — | `null` | Removes all elements from the set in place. |

### 5.8 `matrix` (`PilotMatrix`)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) and [`std.numpy`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/numpy/numpy.pilot):
- Constructed via `matrix(rows, cols)`, `eye(size)`, `zeros(rows, cols)`, `ones(rows, cols)`.
- **Supported Operators**:
  - `A + B`: Element-wise matrix addition (throws if dimensions differ).
  - `A - B`: Element-wise matrix subtraction (throws if dimensions differ).
  - `A * B`: Matrix-matrix dot product ($m \times k$ multiplied by $k \times n$ yields $m \times n$).
  - `A * scalar`: Multiplies every element of matrix `A` by numeric `scalar`.
  - `A == B`: Structural equality comparing dimensions and elements within `1e-9` tolerance.
  - `len(A)`: Returns `Rows * Cols`.

### 5.9 `range` (`PilotRange`)
- Constructed via `start..end` (half-open), `start..=end` (inclusive), or `range(start, end, step)`.
- `len(r)` returns the total number of integers in the range.
- Directly iterable in `for i in 0..10` and unpackable via spread `[..0..5]` -> `[0, 1, 2, 3, 4]`.

### 5.10 `MethodGroup` (`PilotMethodGroup`)
Implemented in [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm) and [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis):

| Member / Operator | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`mg.name`** | `str` *(property)* | Name of the method group (or `"A\|B"` for union groups). |
| **`mg.overloads`** | `list<OverloadInfo>` *(property)* | Returns a list of reflection objects, one per overload in the group. Each `OverloadInfo` has:<br>- `.name` (`str`)<br>- `.signature` (`str`, e.g., `"(x: int) -> any"`)<br>- `.return_type` (`str`)<br>- `.is_async` (`bool`)<br>- `.is_generator` (`bool`)<br>- `.parameters` / `.args` (`list<map>` with `"name"`, `"type"`, `"is_params"`). |
| **`mg.broadcast(args...)`** | `fn -> list` | Invokes **all** matching overloads in the group with `args` and returns a `list` of their return values (identical to `mg*(args...)`). |
| **`mg1 \| mg2`** | `MethodGroup` | Merges two method groups or functions into a unified fallback overload set. |
| **`mg(partial_args)`** | `MethodGroup` | **Signature Sieve Currying**: When called with partial positional or named arguments that do not completely satisfy an overload, filters the group down to compatible overloads with those arguments pre-bound. |

### 5.11 `Generator` (`PilotGenerator`)
Implemented in [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`gen.next()`** | `fn() -> any` | Resumes the generator with `undefined` input and returns the next yielded value (or `undefined` if completed). |
| **`gen.send(val)`** | `fn(any) -> any` | Sends `val` into the suspended `yield` expression, resumes execution to the next `yield`, and returns the yielded value. |
| **`gen.close()`** | `fn() -> null` | Transitions generator state to `Closed`, cancels any active runner thread, and invokes cleanup callbacks. |
| **`gen.is_done`** | `bool` *(property)* | `true` if `State` is `Completed` or `Closed`, otherwise `false`. |
| **`gen.state`** | `str` *(property)* | Current lifecycle state string: `"suspended"`, `"running"`, `"completed"`, or `"closed"`. |

### 5.12 `Lazy` (`PilotLazy`)
Implemented in [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`l.value`** | `any` *(property)* | Evaluates the underlying thunk on first access (thread-safe via lock), caches the result, and returns it on all subsequent accesses. |
| **`l.eval()`** | `fn() -> any` | Method form of `.value`; forces evaluation (if not yet evaluated) and returns the cached value. |
| **`l.is_evaluated`** | `bool` *(property)* | `true` if the lazy thunk has already been evaluated and cached, `false` otherwise. |
| **`l.<any_member>`** | `any` | Accessing any other member on a `PilotLazy` automatically forces evaluation and forwards the member lookup to the inner `Value`. |

### 5.13 `Future` (`PilotFuture`)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) and [`std.asyncio`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/asyncio/asyncio.pilot):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`f.await()`** | `fn() -> any` | Blocks until the background task completes and returns its `PilotValue` result (identical to `await f`). |
| **`f.get_result(timeout_ms?)`** | `fn(int?) -> any` (alias `f.get`) | Waits up to `timeout_ms` milliseconds for the task to complete (waits indefinitely if omitted or `< 0`). Throws `TimeoutException` if the timeout expires. |
| **`f.cancel()`** | `fn() -> null` | Requests cancellation on the task's `CancellationTokenSource`. |
| **`f.is_completed`** | `bool` *(property)* | `true` if the task has finished execution. |
| **`f.is_faulted`** | `bool` *(property)* | `true` if the task terminated with an unhandled exception. |

### 5.14 `Object` (`PilotObject` Class Instances)
Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) ([`value.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/value.rs)) and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm):

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`obj.current_state()`** | `fn() -> str \| null` (alias `obj.currentState()`) | Returns the name of the active typestate (e.g., `"Disconnected"`, `"Connected"`), or `null` if the class does not use typestates. |
| **`obj.transition_to(newState)`** | `fn(str) -> Object` | Transitions the instance's `CurrentState` to `newState` and returns `self`, unlocking methods in the target `state` block and locking out methods exclusive to previous states. |
| **`__init__(self, ...)`** | Constructor | Automatically invoked when instantiating `ClassName(...)` (also supports `constructor(self, ...)` or a method matching `ClassName`). |

### 5.15 `ClassNode` (`PilotClassNode` Generative AST Reflection)
Implemented in [`pilot-ast`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ast) and [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis). Passed as the `ast` argument to macros defined inside a class body:

| Member / Method | Type | Exact Behavior |
| :--- | :--- | :--- |
| **`ast.name`** | `str` *(property)* | Name of the class being compiled/constructed. |
| **`ast.fields`** | `list<FieldNode>` *(property)* | List of field metadata objects on the class. Each field object exposes `.name` (`str`), `.type` (`str`), and `.default_value` (`any`). |
| **`ast.add_field(name, default_value?)`** | `fn(str, any?) -> null` | Dynamically injects a new field into the class blueprint with an optional default value. |
| **`ast.add_method(name, body)`** | `fn(str, str) -> null` | Dynamically synthesizes and injects a new method (such as `"return self.<field>"` getters) onto the class blueprint. |

---

## 6. Design-by-Contract Intrinsics (`old` & `result`)

Inside `ensures` postcondition clauses on functions:

| Intrinsic | Context | Exact Behavior |
| :--- | :--- | :--- |
| **`old(expr)`** | `ensures` clause | Evaluates `expr` in the pre-execution scope before the function body executes, capturing a snapshot. When postconditions execute after completion, `old(expr)` yields that pre-state snapshot. |
| **`result`** | `ensures` clause | Automatically bound in the function's postcondition scope to the return value produced by the function body (e.g., `ensures result > 0`). |

---

## 7. Standard Contracts & Delegated Interfaces

When a class delegates a contract to a field or substitute (`IList.Contracts = _items`, `contracts for IList = _items`, or `delegate IList to _items`), Pilot automatically synthesizes forwarding methods for all methods in that contract not explicitly overridden on the class:

| Contract Name | Synthesized Forwarding Methods | Special Behavior |
| :--- | :--- | :--- |
| **`IList`** | `add`, `append`, `remove`, `pop`, `count`, `length`, `get`, `set`, `clear`, `contains`, `insert` | Directly dispatches to native list operations or custom object methods. |
| **`IEquatable`** | `equals` | Compares the delegated key/member on `self` against the corresponding delegated key/member on `other`. |
| **`IComparable`** | `compare_to`, `compareTo` | Forwards comparison calls to the delegated target member. |
| **`IMap` / `IDictionary`** | `get`, `set`, `contains_key`, `remove`, `keys`, `values`, `count`, `clear` | Directly dispatches to native map operations or custom object methods. |
| **`ISet`** | `add`, `remove`, `contains`, `count`, `clear` | Forwards set operations to the delegated target member. |
| **`IDisposable`** | `dispose` | Forwards resource cleanup to the delegated target member. |
| **`IEnumerable`** | `get_enumerator`, `__iter__` | Forwards iteration protocol calls to the delegated target member. |
| **Any User `interface` / `class`** | All method names defined on that interface/class in scope | Dynamically resolved from scope and forwarded to the target member. |

---

## 8. Built-in Type Adapters (`PilotTypeAdapterRegistry`)

Implemented in [`pilot-analysis`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis). Whenever a function parameter has a type annotation `T` and receives a value of type `S`, Pilot automatically applies the matching adapter below (in addition to any user-defined `adapter S -> T` rules):

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
| `list` | `tuple` | Wraps list elements in an immutable tuple value. |
| `tuple` | `list` | Copies tuple elements into a mutable list. |
| `list` | `set` | Copies list elements into a unique set. |
| `set` | `list` | Copies set elements into a list. |
| `dict` / `map` | `object` | Instantiates an object and populates all map entries as object fields. |
| `map` | **Any User Class `C`** | **Structural Hydration**: Resolves class `C` in scope, instantiates `C`, and copies every key-value pair from the `map` into the instance's fields. |

---

## 9. Polyglot Provider Built-in Functions (`PolyglotRegistry`)

Implemented in [`pilot-polyglot`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-polyglot):
- **Python Provider**:
  - Evaluates inline `python { ... }` blocks, binding variables from the enclosing Pilot scope and supporting `yield` coroutines.
  - Direct member invocation (`Invoke`): `sin(x)`, `cos(x)`, `tan(x)`, `sqrt(x)`, `pi`, `e`, `dict()`, `list()`.
- **C# Provider**:
  - Evaluates inline `csharp { ... }` blocks.
  - Direct member invocation (`Invoke`): `NewGuid()`, `Now`, `Sin(x)`, `Cos(x)`, `Abs(x)`.
- **JavaScript (`js`), Java (`java`), Lua (`lua`), C (`c`), C++ (`cpp`) Providers**:
  - Evaluate `return <expr>;` statements, variable lookups from Pilot scope, and standard output calls (`console.log(...)`, `System.out.println(...)`, `print(...)`, `printf(...)`, `std::cout << ...`).
- **Assembly Provider**:
  - Simulates 32/64-bit x86/x64 registers (`eax`, `ebx`, `ecx`, `edx`, `rax`, `rbx`, `rcx`, `rdx`), executing `mov dest, src`, `add dest, src` (reading immediate values, registers, or Pilot scope variables), and `ret`, returning the final value of `eax` / `rax`.
- **PostgreSQL (`postgresql`) & MySQL (`mysql`) Providers**:
  - Execute SQL `SELECT` queries (including `WHERE id IN (...)` filtering) and return a `list` of structured `PilotObject` rows with `.id`, `.name`, and `.count` fields, or return affected row counts (`1`) for DML commands.

---

## 10. Runtime Exception Hierarchy (`PilotExceptions`)

Implemented in [`pilot-rt`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt) and [`pilot-vm`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm). All runtime errors inherit from `RuntimeError` and can be caught in `try ... except <TypeName> as err` blocks:

| Pilot Exception Name | Native Runtime Representation | Trigger Condition |
| :--- | :--- | :--- |
| **`Exception` / `Error` / `RuntimeError`** | Base runtime error | Base class catching any runtime exception in Pilot. |
| **`TypeError`** | Type mismatch fault | Type mismatch, no matching overload in a method group, calling a non-function, or calling a typestate method in the wrong state. |
| **`DivideByZeroError`** | Math divide-by-zero fault | Division (`/`, `//`) or modulo (`%`) by `0` or `0.0`. |
| **`IndexError`** | Bounds check fault | Out-of-bounds index on a matrix or strict sequence. |
| **`KeyError`** | Map key missing fault | Missing required key in dictionary/map operations. |
| **`NullReferenceError`** | Null dereference fault | Accessing `.member` on `null` or `undefined` without safe navigation `?.`. |
| **`AssertionError`** | Assertion failure | `assert condition` statement evaluated to `false`. |
| **`RecursionError`** | Call stack overflow | Call stack depth exceeded configured recursion limit. |
| **`ContractError`** | Contract verification fault | Design-by-Contract `requires` precondition or `ensures` postcondition evaluated to `false`. |
| **`TimeoutError`** | Operation timeout | Asynchronous future or concurrent operation exceeded its deadline/timeout. |
| **`MatchError`** | Exhaustiveness/destructure fault | Destructuring assignment (`[a, b] = ...` or `{ "k": v } = ...`) failed to match the runtime structure or arity. |
| **`ContextError`** | Missing context frame | A required `context` parameter could not be resolved from any enclosing `with context` block. |
| **`UserError` (or custom class name)** | User payload object | Explicitly thrown by a user `raise expr` statement; preserves user payload when caught via `except as err`. |

---

## 11. Standard Library Modules & Native Resolvers (`stdlib/`)

Pilot ships with **40 foundational standard library modules** located in `stdlib/`. These modules provide zero-external-dependency, strictly typed implementations covering low-level memory control, concurrent event loops, data science pipelines, POSIX/Windows system APIs, and cryptographic hashing.

### 11.1 Module Architecture & Provenance

Standard library modules are defined in `stdlib/pilot.json` and are fully validated under permissive open-source licenses (MIT, PSF-2.0, and BSD-3-Clause). Modules can be imported in any Pilot source file via the `import` statement:

```pilot
import std.collections
import std.math
import std.json
import std.asyncio
```

### 11.2 Core & Memory

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.core`** | `core/core.pilot` | `min`, `max`, `abs`, `some`, `none`, `all`, `any`, `enumerate`, `zip`, `reversed` | Foundational primitives, functional collection predicates, and root type utilities. |
| **`std.option`** | `core/option.pilot` | `Option[T]`, `Some(v)`, `None()`, `unwrap()`, `unwrap_or(default)`, `map(fn)` | Type-safe optional container preventing null pointer errors. |
| **`std.result`** | `core/result.pilot` | `Result[T, E]`, `Ok(v)`, `Err(e)`, `is_ok()`, `is_err()`, `unwrap()`, `map_err(fn)` | Monadic error handling construct for robust control flow. |
| **`std.mem`** | `core/mem.pilot` | `size_of[T]()`, `align_of[T]()`, `zeroed[T]()`, `copy(src, dst, count)` | Low-level memory layout queries, zeroing, and direct memory transfers. |
| **`std.meta`** | `core/meta.pilot` | `type_name(val)`, `fields(obj)`, `methods(obj)`, `has_field(obj, name)` | Compile-time and runtime type reflection and structural introspection. |

### 11.3 Collections & Iteration

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.collections`** | `collections/collections.pilot` | `Counter`, `defaultdict`, `OrderedDict`, `ChainMap`, `deque`, `namedtuple`, `bisect_left`, `bisect_right`, `PriorityQueue`, `BitSet` | High-performance containers, frequency counters, double-ended queues, binary min-heaps, and SWAR 64-bit bitsets. |
| **`std.itertools`** | `collections/itertools.pilot` | `chain`, `batched`, `compress`, `dropwhile`, `filterfalse`, `groupby`, `islice`, `pairwise`, `starmap`, `takewhile`, `product`, `permutations`, `combinations` | Zero-allocation iterator combinators, Cartesian products, groupings, and combinations. |
| **`std.text`** | `text/text.pilot` | `StringBuilder`, `dedent`, `indent`, `wrap`, `fill`, `slugify` | High-throughput mutable string concatenation, text formatting, and wrapping. |

### 11.4 Mathematics & Randomness

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.math`** | `math/math.pilot` | `PI`, `TAU`, `E`, `SQRT2`, `deg_to_rad`, `rad_to_deg`, `sigmoid`, `gcd`, `lcm`, `factorial`, `hypot`, `clamp`, `lerp`, `welford` | Differentiable elementary functions, number theory, trigonometry, and online Welford statistics. |
| **`std.random`** | `math/random.pilot` | `seed(n)`, `random()`, `randint(a, b)`, `choice(seq)`, `shuffle(seq)`, `sample(pop, k)`, `gauss(mu, sigma)` | Cryptographically seeded pseudorandom number generators and Gaussian distributions. |
| **`std.decimal`** | `decimal/decimal.pilot` | `Decimal`, `getcontext()`, `quantize()`, `ROUND_HALF_UP` | Arbitrary-precision fixed-point and floating-point arithmetic for financial operations. |

### 11.5 I/O & File System

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.io`** | `io/io.pilot` | `StringIO`, `BytesIO`, `TextIOWrapper`, `SEEK_SET`, `SEEK_CUR`, `SEEK_END` | In-memory text and binary stream buffers and seek primitives. |
| **`std.fs`** | `io/fs.pilot` | `open(path, mode)`, `read(path)`, `write(path, content)`, `exists(path)`, `remove(path)`, `listdir(path)` | Direct file system access, buffered file handles, and directory traversal. |
| **`std.path`** | `io/path.pilot` | `join(*parts)`, `split(path)`, `basename(path)`, `dirname(path)`, `abspath(path)`, `isabs(path)` | Cross-platform file path manipulation, normalizations, and extensions. |
| **`std.pathlib`** | `io/pathlib.pilot` | `Path`, `PurePath`, `Path.cwd()`, `Path.home()`, `exists()`, `is_file()`, `is_dir()`, `read_text()`, `write_text()` | Object-oriented path representations with operator overloading (`/` joining). |
| **`std.shutil`** | `io/shutil.pilot` | `copy(src, dst)`, `copytree(src, dst)`, `rmtree(path)`, `move(src, dst)`, `which(cmd)`, `disk_usage(path)` | High-level file and directory management, recursive copying/deletion, and PATH resolution. |

### 11.6 Operating System & Process

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.os`** | `os/os.pilot` | `environ`, `getenv(k, default)`, `putenv(k, v)`, `getcwd()`, `chdir(path)`, `getpid()`, `sep`, `linesep` | Host operating system interaction, environment variables, working directory, and process ID. |
| **`std.sys`** | `sys/sys.pilot` | `argv`, `platform`, `byteorder`, `version`, `exit(code)`, `stdin`, `stdout`, `stderr` | Toolchain runtime metadata, command line arguments, and standard I/O stream descriptors. |

### 11.7 Functional Programming

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.functools`** | `functools/functools.pilot` | `partial(fn, *args)`, `reduce(fn, seq, init)`, `lru_cache(maxsize)`, `cache(fn)`, `singledispatch(fn)`, `cmp_to_key(fn)` | Higher-order functional adapters, memoization caches, and single-dispatch generic functions. |

### 11.8 Serialization & Formats

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.json`** | `json/json.pilot` | `dumps(obj, indent)`, `loads(json_str)`, `dump(obj, fp)`, `load(fp)` | RFC 8259 compliant JSON serializer and deserializer. |
| **`std.csv`** | `csv/csv.pilot` | `reader(lines, delimiter)`, `writer(fp, delimiter)`, `DictReader(lines)`, `DictWriter(fp, fieldnames)` | Tabular CSV and TSV reading and writing with customizable delimiters and quote handling. |
| **`std.pickle`** | `pickle/pickle.pilot` | `dumps(obj)`, `loads(bytes)`, `dump(obj, fp)`, `load(fp)` | Fast binary object serialization protocol for Pilot object graphs. |
| **`std.tomllib`** | `tomllib/tomllib.pilot` | `loads(toml_str)`, `load(fp)` | TOML (Tom's Obvious Minimal Language) configuration parser. |
| **`std.encoding`** | `encoding/encoding.pilot` | `b64encode(b)`, `b64decode(s)`, `hex_encode(b)`, `hex_decode(s)`, `utf8_encode(s)`, `utf8_decode(b)` | Base64, Hexadecimal, and UTF-8 conversion encoders and decoders. |
| **`std.formats`** | `formats/formats.pilot` | `format_number(n, spec)`, `format_table(headers, rows)`, `parse_hex(s)` | Formatted text presentation, ASCII table rendering, and numeric conversions. |

### 11.9 Concurrency & Asynchronous

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.threading`** | `threading/threading.pilot` | `Thread(target, args)`, `Lock()`, `RLock()`, `Semaphore(value)`, `Event()`, `current_thread()` | OS-backed threads, synchronization primitives, reentrant mutexes, and signals. |
| **`std.multiprocessing`** | `multiprocessing/multiprocessing.pilot` | `Process(target, args)`, `Pool(processes)`, `cpu_count()`, `Queue()` | Multi-process parallelism across distinct address spaces with worker pools. |
| **`std.asyncio`** | `asyncio/asyncio.pilot` | `Future`, `Task`, `EventLoop`, `run(coro)`, `gather(*coros)`, `sleep(delay)`, `wait_for(fut, timeout)` | Non-blocking asynchronous event loop, coroutine scheduling, futures, and timeouts. |

### 11.10 Networking & HTTP

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.socket`** | `socket/socket.pilot` | `Socket(family, type)`, `AF_INET`, `SOCK_STREAM`, `SOCK_DGRAM`, `bind()`, `listen()`, `connect()`, `send()`, `recv()` | Low-level TCP and UDP socket networking interface across IPv4 and IPv6. |
| **`std.urllib`** | `urllib/urllib.pilot` | `urlparse(url)`, `urlunparse(parts)`, `urlencode(query)`, `quote(s)`, `unquote(s)` | URL syntax decomposition, query parameter encoding, and percent-escaping. |
| **`std.http`** | `http/http.pilot` | `HTTPStatus`, `HTTPConnection(host, port)`, `request(method, url, headers, body)`, `getresponse()` | Multi-OS HTTP/1.1 client connection engine supporting persistent connections. |
| **`std.request`** | `request/request.pilot` | `get(url, params, headers)`, `post(url, data, json, headers)`, `put()`, `delete()`, `Response` | High-level ergonomic REST request library with automatic JSON parsing and status assertions. |

### 11.11 Data Science & Scientific Computing

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.numpy`** | `numpy/numpy.pilot` | `ndarray`, `zeros(shape)`, `ones(shape)`, `arange(start, stop, step)`, `linspace(a, b, num)`, `dot(a, b)`, `cross(a, b)`, `norm(a)`, `mean(a)`, `std(a)`, `sum(a)` | High-performance N-dimensional array processing, matrix algebra, and vector math. |
| **`std.pandas`** | `pandas/pandas.pilot` | `Series(data, index)`, `DataFrame(data, columns, index)`, `read_csv(path)`, `to_csv(path)`, `head(n)`, `tail(n)`, `describe()`, `groupby(col)` | Relational data manipulation, tabular DataFrames, labeled Series, and group-by aggregations. |
| **`std.matplotlib`** | `matplotlib/matplotlib.pilot` | `plot(x, y, fmt)`, `scatter(x, y)`, `bar(x, heights)`, `title(t)`, `xlabel(l)`, `ylabel(l)`, `show()`, `savefig(path)` | Plotting and visualization pipeline emitting charts and vector graphics. |
| **`std.scikit`** | `scikit/scikit.pilot` | `LinearRegression`, `LogisticRegression`, `KMeans(k)`, `StandardScaler`, `train_test_split(X, y)` | Machine learning algorithms: ordinary least squares, classification, clustering, and data preprocessing. |

### 11.12 Testing, Profiling & Security

| Module | Source Path | Key Classes & Functions | Description |
| :--- | :--- | :--- | :--- |
| **`std.unittest`** | `unittest/unittest.pilot` | `TestCase`, `assertEqual(a, b)`, `assertTrue(x)`, `assertFalse(x)`, `assertRaises(err, fn)`, `main()` | Automated test runner with test discovery, fixtures (`setUp`, `tearDown`), and assertions. |
| **`std.logging`** | `logging/logging.pilot` | `Logger`, `DEBUG`, `INFO`, `WARNING`, `ERROR`, `CRITICAL`, `basicConfig(level, format)`, `info(msg)`, `error(msg)` | Structured logging with configurable log levels, formatters, and stream destinations. |
| **`std.timeit`** | `timeit/timeit.pilot` | `timeit(stmt, number)`, `repeat(stmt, repeat, number)` | High-resolution micro-benchmarking harness for statement execution timing. |
| **`std.secrets`** | `secrets/secrets.pilot` | `token_hex(nbytes)`, `token_urlsafe(nbytes)`, `randbelow(n)`, `choice(seq)` | Cryptographically strong pseudo-random number generator for security tokens and passkeys. |
| **`std.crypto`** | Native (`pilot-rt`) | `sha256(bytes)`, `sha512(bytes)`, `blake3(bytes)`, `hmac_sha256(key, data)`, `ed25519_sign(sk, msg)`, `ed25519_verify(pk, msg, sig)` | High-throughput cryptographic hashing and Ed25519 public-key signature verification. |
| **`std.time`** | Native (`pilot-rt`) | `now_millis()`, `now_nanos()`, `monotonic_micros()`, `sleep_ms(ms)`, `timespec_now()` | High-precision hardware monotonic timers and real-time execution sleep primitives. |

### 11.13 Module Resolver & Precompiled Bytecode Cache (`PilotStdlibResolver`)

Standard library modules are discovered and loaded through `pilot_rt::PilotStdlibResolver`:
1. **Cache Verification (`stdlib/.pilc-cache/`)**:
   Checks whether a precompiled bytecode binary (`std.<module>.pilc` or `std.<module>_<sha256>.pilc`) exists and matches the source hash. If present, it loads with zero deserialization overhead ($< 0.2\text{ ms}$).
2. **On-Demand Precompilation (`pilot compile --stdlib`)**:
   Compiles all 40 standard library modules directly into the `.pilc-cache/` directory:
   ```bash
   pilot compile --stdlib
   ```
3. **Source Fallback**:
   If no cache entry exists, `PilotStdlibResolver` maps `std.<module>` to its relative path under `stdlib/` (as specified in `stdlib/pilot.json`) and parses the source file directly.

### 11.14 Linkable C-ABI Native Runtime API

The Pilot runtime crate (`pilot-rt`) produces native static and dynamic libraries for direct linking with native C, C++, or Rust applications:
- **`libpilot_rt.a`** (Static Library): Complete native runtime with fat LTO, Perceus reference counting, and bump allocators.
- **`libpilot_rt.so`** (Shared Dynamic Library): Exported standard C-ABI symbols.
- **`pilot_runtime.h`** (C Header):
  ```c
  #include "pilot_runtime.h"

  // Region Memory Allocation (Tier 1)
  void* pilot_rt_region_enter(void);
  void* pilot_rt_region_alloc(void* region, size_t size, size_t align);
  void  pilot_rt_region_exit(void* region);

  // Perceus Reference Counting (Tier 2)
  void* pilot_rt_alloc_arc(size_t payload_size);
  void* pilot_rt_dup(void* ptr);
  void  pilot_rt_drop(void* ptr);
  void* pilot_rt_drop_reuse(void* ptr, size_t required_size);

  // Time & Cryptography
  uint64_t pilot_rt_time_now_ns(void);
  void     pilot_rt_sleep_ms(uint64_t ms);
  void     pilot_rt_crypto_sha256(const uint8_t* data, size_t len, uint8_t* out_32);
  void     pilot_rt_crypto_random_bytes(uint8_t* out_buf, size_t len);
  ```

---

[**← Syntax Reference**](syntax-reference.md) | [**Documentation Hub**](index.md) | [**Next: Concept 01 — Types, Variables & Control Flow →**](concepts/01-types-variables-control-flow.md)
