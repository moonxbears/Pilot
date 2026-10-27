# Concept 02: Guard, Defer & Structured Exception Handling

[← Prev: Types, Variables & Control Flow](./01-types-variables-control-flow.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Method Groups & Dispatch →](./03-method-groups-and-dispatch.md)

---

## 1. Concept Overview & Design Rationale

Pilot combines three complementary mechanisms to keep error handling and resource cleanup flat, predictable, and leak-free:

1. **`guard ... else` Early-Exit Preconditions**: Inspired by Swift, `guard` enforces invariants or unpacks patterns at the top of a scope. If the condition or pattern fails, the `else` block executes and **must** transfer control out of the current flow (`return`, `break`, `continue`, or `raise`), eliminating deeply nested "arrow-code" `if` pyramids.
2. **`defer` LIFO Cleanup Stack**: Inspired by Go and Zig, `defer` registers cleanup code directly next to resource acquisition. Deferred blocks are guaranteed to run in **Last-In, First-Out (LIFO)** order when the enclosing function (or top-level script) exits—even if an exception is raised or an early `return` is taken.
3. **`try` / `except` / `else` / `finally` & `with` Context Managers**: Full structured exception handling with type-filtered catch clauses (`except DivideByZeroError as err`), an `else` branch that runs only when no exception occurred, a `finally` block, and deterministic `with` resource management (`close()` / `dispose()`).

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `GUARD`, `ELSE` | `guard`, `else` | Precondition invariant check or pattern-binding guard |
| `LET`, `VAR` | `let`, `var` | Pattern-binding form of `guard` (`guard let ... = ... else`) |
| `DEFER` | `defer` | Schedules a block or statement for LIFO scope-exit execution |
| `TRY`, `EXCEPT`, `FINALLY` | `try`, `except`, `finally` | Structured exception handling blocks |
| `AS` | `as` | Binds the caught exception message or `with` resource variable |
| `RAISE` | `raise` | Raises a runtime `UserError` (`PilotUserException`) |
| `ASSERT` | `assert` | Evaluates an invariant and raises `AssertionError` on failure |
| `WITH`, `ASYNC` | `with`, `async` | Deterministic resource management block |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// Guard Statement (Boolean Condition or Pattern Unpacking)
guard_stmt
    : GUARD expr ELSE suite
    | GUARD (LET | VAR) pattern_target ASSIGN expr ELSE suite
    ;

// Defer Statement (Suite Block or Single Inline Statement)
defer_stmt
    : DEFER (suite | small_stmt NEWLINE?)
    ;

// Structured Exception Handling
try_stmt
    : TRY suite
      ( (except_clause suite)+ (ELSE suite)? (FINALLY suite)?
      | FINALLY suite
      )
    ;

except_clause
    : EXCEPT (expr (AS IDENTIFIER)?)?
    ;

// Raise & Assert Statements
raise_stmt
    : RAISE (expr (FROM expr)?)?
    ;

assert_stmt
    : ASSERT expr (COMMA expr)?
    ;

// Resource Management With Statement
with_stmt
    : ASYNC? WITH with_item (COMMA with_item)* suite
    ;

with_item
    : expr (AS expr)?
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 `guard` Execution (`VisitGuard_stmt`)
In [`PilotInterpreterVisitor.cs`](../../Runtime/Execution/PilotInterpreterVisitor.cs):
1. **Boolean Guard (`guard expr else suite`)**: Evaluates `expr`. If `condition.IsTruthy()` is `true`, execution continues to the next statement. If `false`, the `else` `suite` is executed.
2. **Pattern Guard (`guard let pattern = expr else suite`)**: Evaluates `expr` and runs `PilotPatternMatcher.BindPatternTarget(pattern, val, Context, isConst: false)`.
   - If the pattern matches, all extracted variables are bound directly into the **current enclosing scope**, and execution continues.
   - If `BindPatternTarget` throws `PilotPatternMatchingException` or if the single bound variable evaluates to `null`/`none`/`undefined`, the exception is caught and the `else` `suite` executes.

### 4.2 `defer` LIFO Stack (`PushDefer` & `ExecuteDefers`)
`PilotExecutionContext` maintains two LIFO `Stack<Action>` queues:
- **Inside a Function Call**: `PushFrame` initializes a per-frame `_deferStack`. Each `defer` statement pushes an `Action` closure (`() => Visit(body)`) onto the current frame's stack. When `PopFrame()` runs inside the `finally` block of `CallFunction`, `ExecuteDefers()` pops and invokes every deferred action in reverse registration order.
- **At the Global Script Level**: If `defer` is executed outside any function, `Context.PushDefer` pushes onto `_globalDefers`, which `ExecuteGlobalDefers()` drains inside the `finally` block of `VisitProgram`.

```mermaid
sequenceDiagram
    participant Fn as Function Body
    participant Stack as Frame Defer Stack (LIFO)
    participant Exit as PopFrame (finally)

    Fn->>Stack: defer { log("1. Close DB") }
    Fn->>Stack: defer { log("2. Flush Cache") }
    Fn->>Fn: Execute main logic / return / raise
    Fn->>Exit: Scope exits (normal or exception)
    Exit->>Stack: Pop & Execute ("2. Flush Cache")
    Exit->>Stack: Pop & Execute ("1. Close DB")
```

### 4.3 Exception Hierarchy & `try`/`except` Matching
All runtime errors inherit from `PilotRuntimeException` ([`PilotExceptions.cs`](../../Runtime/ControlFlow/PilotExceptions.cs)), which carries an `ExceptionType` string identifier:

| Exception Type Name | C# Exception Class | Trigger Condition |
| :--- | :--- | :--- |
| `TypeError` | `PilotTypeException` | Invalid operand types, typestate violation, invalid call target |
| `DivideByZeroError` | `PilotDivideByZeroException` | `/`, `//`, or `%` with a zero divisor |
| `IndexError` | `PilotIndexOutOfRangeException` | Out-of-bounds list, tuple, string, or matrix index |
| `KeyError` | `PilotKeyNotFoundException` | Missing key lookup on a `map` |
| `NullReferenceError` | `PilotNullReferenceException` | Member access or call on `null`/`none`/`undefined` without `?.` |
| `AssertionError` | `PilotAssertionException` | `assert` statement or `assert()` call failed |
| `RecursionError` | `PilotRecursionLimitException` | Call stack depth exceeded `MaxCallDepth` (`256`) |
| `ContractError` | `PilotContractException` | `requires`, `ensures`, or contract interface violation |
| `TimeoutError` | `PilotTimeoutException` | `Future.get_result(ms)` or `bounded` loop limit exceeded |
| `MatchError` | `PilotPatternMatchingException` | Destructuring pattern mismatch |
| `ContextError` | `PilotContextException` | Unbound `context` parameter with no default value |
| `UserError` | `PilotUserException` | Raised explicitly by `raise expr` (carries `.ThrownValue`) |

When an `except TypeName as err:` clause is evaluated:
- An bare `except:` catches any `PilotRuntimeException`.
- `except DivideByZeroError as err:` matches if the identifier (`DivideByZeroError`) equals `ex.ExceptionType`, equals `"Exception"` or `"Error"`, or matches the thrown value's type/string.
- If `as err` is specified, `err` is bound to `ex.ThrownValue` (for `PilotUserException` when non-null) or `PilotValue.FromString(ex.Message)`.

---

## 5. Available Functions, Methods & Properties

| Construct / Function | Signature | Description |
| :--- | :--- | :--- |
| `assert` (statement) | `assert cond [, message]` | Evaluates `cond`; raises `AssertionError` with optional `message` if falsy. |
| `assert` (function) | `assert(cond[, message]) -> none` | Functional form of `assert` usable inside expressions or pipelines. |
| `raise` (statement) | `raise value` | Wraps `value` in a `PilotUserException` (`UserError`) and unwinds the stack. |
| `IDisposable` cleanup | `with expr as var { ... }` | Automatically invokes `close()` or `dispose()` on `PilotObject` instances when exiting the `with` block. |

---

## 6. Complete Runnable Examples

See [`examples/showcase/02_guard_defer_and_exceptions.pilot`](../../examples/showcase/02_guard_defer_and_exceptions.pilot) for the full executable showcase.

```pilot
let audit_trail = []

def process_order(payload: map) -> str {
    // Register LIFO defers: guaranteed to run in reverse order on ANY exit
    defer { audit_trail.append("step-3: release connection") }
    defer { audit_trail.append("step-2: flush metrics") }

    // 1. Pattern-binding guard: unpacks 'order_id' and 'amount' into scope
    guard let {"order_id": id, "amount": amt} = payload else {
        audit_trail.append("step-1: rejected malformed payload")
        return "INVALID_PAYLOAD"
    }

    // 2. Boolean invariant guard
    guard amt > 0 else {
        raise f"Invalid order amount: {amt}"
    }

    audit_trail.append(f"step-1: processed #{id}")
    return f"OK:{id}"
}

// Normal execution
let status = process_order({"order_id": 101, "amount": 50})
assert(status == "OK:101")
// audit_trail is now:
// ["step-1: processed #101", "step-2: flush metrics", "step-3: release connection"]

// Exception handling with try / except / else / finally
let caught_msg = ""
let finally_ran = false

try {
    process_order({"order_id": 102, "amount": -10})
} except UserError as err {
    caught_msg = err
} finally {
    finally_ran = true
}

assert(caught_msg == "Invalid order amount: -10")
assert(finally_ran == true)
```

---

[← Prev: Types, Variables & Control Flow](./01-types-variables-control-flow.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Method Groups & Dispatch →](./03-method-groups-and-dispatch.md)
