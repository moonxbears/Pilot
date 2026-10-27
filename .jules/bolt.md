
## 2026-10-27 - Buffered Reading in LSP Server

**Learning:** When writing an LSP server (or any text-protocol server reading from a network stream), using a 1-byte read (`await input.ReadAsync(oneByte, ct)`) to search for message boundaries like `\r\n\r\n` causes catastrophic overhead due to state-machine allocations and context switches on every byte.
**Action:** Always maintain an internal buffer (e.g. `byte[8192]`) and manage buffer offsets across asynchronous message boundaries instead of reading byte-by-byte. For bodies that exceed the buffer's remaining capacity, bypass the buffer to avoid unnecessary memory copies.
## 2024-10-27 - [Iterative variable resolution in lexical scope hierarchy]
**Learning:** In heavily nested variable scopes (like those common when defining many local scopes/blocks), recursive calls to resolve variables or assign values (e.g., `Parent.TryResolve(name, out value)`) introduce stack overhead and can be measurably slower than an iterative `while(current != null)` loop over the parent chain. This applies to operations executed constantly by the interpreter.
**Action:** Always favor iterative traversal of hierarchical structures (like scopes or prototypes) over recursion for frequently called methods like `TryResolve` or `Assign`, especially in a dynamic language execution engine.
