# Pilot 8-Language Bidirectional Round-Trip Samples

This folder contains the persisted artifacts from emitting [`00_source.pilot`](./00_source.pilot) across all **8 target languages** (`pilot --emit <lang>`) and ingesting every emitted file back into Pilot (`pilot --from <lang>`).

## 1. Canonical Pilot Input
- [`00_source.pilot`](./00_source.pilot) — Exercises `enum`, `type` ADTs, `extend`, `class` methods, typed functions, `while` / `for i in a..b`, `if` / `elif` / `else`, operator precedence, filtered list comprehensions `[x * x for x in 1..6 if x > 2]`, `f"..."` string interpolation, and `assert`.

## 2. Emitted Foreign Target Files (`pilot --emit <lang>`)
| Target | CLI Flag | Emitted File |
| :--- | :--- | :--- |
| Python 3.10+ | `--emit python` | [`emitted/source.py`](./emitted/source.py) |
| C# (.NET 10) | `--emit csharp` | [`emitted/source.cs`](./emitted/source.cs) |
| Rust (2024) | `--emit rust` | [`emitted/source.rs`](./emitted/source.rs) |
| C (C11/C23) | `--emit c` | [`emitted/source.c`](./emitted/source.c) |
| C++ (C++20/23) | `--emit cpp` | [`emitted/source.cpp`](./emitted/source.cpp) |
| JavaScript (ES2024+) | `--emit js` | [`emitted/source.js`](./emitted/source.js) |
| TypeScript | `--emit ts` | [`emitted/source.ts`](./emitted/source.ts) |
| Lua 5.4 / LuaJIT | `--emit lua` | [`emitted/source.lua`](./emitted/source.lua) |

## 3. Ingested-Back Pilot Files (`pilot --from <lang>`)
| Source Language | CLI Flag | Ingested Pilot File |
| :--- | :--- | :--- |
| Python 3.10+ | `--from python` | [`ingested_back/from_python.pilot`](./ingested_back/from_python.pilot) |
| Python (Strict) | `--from python --strict-pilot` | [`ingested_back/from_python_strict.pilot`](./ingested_back/from_python_strict.pilot) |
| C# | `--from csharp` | [`ingested_back/from_csharp.pilot`](./ingested_back/from_csharp.pilot) |
| Rust | `--from rust` | [`ingested_back/from_rust.pilot`](./ingested_back/from_rust.pilot) |
| C | `--from c` | [`ingested_back/from_c.pilot`](./ingested_back/from_c.pilot) |
| C++ | `--from cpp` | [`ingested_back/from_cpp.pilot`](./ingested_back/from_cpp.pilot) |
| JavaScript | `--from js` | [`ingested_back/from_js.pilot`](./ingested_back/from_js.pilot) |
| TypeScript | `--from ts` | [`ingested_back/from_ts.pilot`](./ingested_back/from_ts.pilot) |
| Lua | `--from lua` | [`ingested_back/from_lua.pilot`](./ingested_back/from_lua.pilot) |
