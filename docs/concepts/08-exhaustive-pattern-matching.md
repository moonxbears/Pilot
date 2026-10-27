# Concept 08: Exhaustive Pattern Matching & Deep Destructuring

[← Prev: Contextual Parameters & DI](./07-contextual-parameters-di.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Design-by-Contract & Stealing →](./09-design-by-contract.md)

---

## 1. Concept Overview & Design Rationale

Pilot treats **Structural Destructuring** and **Pattern Matching** as unified, pervasive primitives across variable declarations, standalone assignments, `guard let` statements, inline `is` tests, and multi-way `match` statements:

- **Deep Nested Unpacking**: Record (`{ "k": pattern }`) and sequence (`[head, ..middle, tail]`) patterns can nest arbitrarily deeply (`{"user": {"name": n, "scores": [first, ..rest]}}`) in a single statement.
- **Default Fallbacks & Rest Capture**: Record patterns support inline default values (`{"role": r = "guest"}`) when keys are missing, and `**extra` captures all remaining unmatched keys into a new `map`. Sequence patterns support `..middle` or `*middle` slice capture at the start, middle, or end of a list.
- **Exhaustive `match` & Rich Pattern Combinators**: `match` arms support literal values, wildcards (`_`), relational bounds (`> 100 and < 500`), disjunctions (`200 | 201 | 204`), negation (`not 0`), typed bindings (`int n`), structural property patterns (`{ status: 200 }`), and `when`/`if` guard clauses.

---

## 2. Exact Lexical Tokens Used

| Token Name | Lexeme(s) | Lexical Category & Purpose |
| :--- | :--- | :--- |
| `MATCH`, `CASE`, `DEFAULT` | `match`, `case`, `default` | Multi-way pattern matching statement and arm keywords |
| `WHEN`, `IF` | `when`, `if` | Guard condition on a `case` arm (`case x when x > 0:`) |
| `IS`, `NOT` | `is`, `not` | Inline pattern test (`val is int n`, `val is not null`) and negation pattern |
| `AND`, `OR`, `PIPE` | `and`, `or`, `\|` | Conjunction and disjunction pattern combinators |
| `RANGE`, `STAR`, `POWER` | `..`, `*`, `**` | Sequence rest capture (`..rest`, `*rest`) and record rest capture (`**rest`) |
| `FAT_ARROW`, `COLON` | `=>`, `:` | Single-line or suite separator for `case` arms |

---

## 3. Exact Syntax & Grammar Rules

From [`Pilot.g4`](../../antlr/Pilot.g4):

```antlr
// 1. Destructuring Declarations & Standalone Assignments
pattern_decl
    : (LET | VAR | CONST) pattern_target ASSIGN testlist_star_expr
    ;

pattern_assign_stmt
    : record_pattern ASSIGN testlist_star_expr
    | list_pattern ASSIGN testlist_star_expr
    ;

pattern_target
    : record_pattern
    | list_pattern
    | IDENTIFIER (COMMA IDENTIFIER)+
    | IDENTIFIER
    ;

record_pattern
    : LBRACE (record_pattern_field (COMMA record_pattern_field)* COMMA?)? RBRACE
    ;

record_pattern_field
    : (STRING | IDENTIFIER) COLON pattern_target (ASSIGN expr)?
    | (STRING | IDENTIFIER) (ASSIGN expr)?
    | POWER IDENTIFIER
    ;

list_pattern
    : LBRACK (list_pattern_elem (COMMA list_pattern_elem)* COMMA?)? RBRACK
    | LPAREN (list_pattern_elem (COMMA list_pattern_elem)* COMMA?)? RPAREN
    ;

list_pattern_elem
    : (RANGE | STAR) IDENTIFIER
    | pattern_target (ASSIGN expr)?
    ;

// 2. Match Statement & Pattern Hierarchy
match_stmt
    : MATCH expr (LBRACE NEWLINE? case_clause* RBRACE | COLON NEWLINE? case_clause* END?)
    ;

case_clause
    : CASE pattern ( (WHEN | IF) expr )? (COLON suite | FAT_ARROW (stmt | expr) COMMA? NEWLINE?)
    | (DEFAULT | CASE IDENTIFIER) (COLON suite | FAT_ARROW (stmt | expr) COMMA? NEWLINE?)
    ;
```

---

## 4. Runtime Execution Mechanics

### 4.1 Deep Destructuring Engine (`PilotPatternMatcher.BindPatternTarget`)
In [`PilotPatternMatcher.cs`](../../Runtime/Execution/PilotPatternMatcher.cs):
- **Record Patterns (`BindRecordPattern`)**:
  - Accepts either a `map` (`Dictionary<PilotValue, PilotValue>`) or a `PilotObject` (`Class` instance).
  - Shorthand `{ name, role = "user" }` looks up key/field `"name"` and binds variable `name`.
  - Explicit `{ "user_id": id, "profile": { "city": city = "UTC" } }` looks up `"user_id"` and recursively matches `"profile"` against the nested pattern.
  - If a key/field is missing and no `= default_expr` is provided, raises `PilotPatternMatchingException` (`MatchError`).
  - If `**rest` is present on a `map`, all keys not explicitly matched by earlier fields are copied into a new `map` and bound to `rest`.
- **Sequence Patterns (`BindListPattern`)**:
  - Accepts `list`, `tuple`, or `range` (materializing ranges into elements).
  - Without a rest element (`..rest` or `*rest`), the collection length must equal the number of pattern elements (unless trailing elements specify `= default_expr`), otherwise `MatchError` is raised.
  - With a rest element (`[first, ..middle, last]`), `beforeCount` leading elements and `afterCount` trailing elements are bound first, and the slice `[beforeCount .. len - afterCount]` is bound to `middle` as a `list`.

### 4.2 `match` & `is` Pattern Evaluation (`MatchPattern`)
`PilotPatternMatcher.MatchPattern` evaluates a `PilotParser.PatternContext` against a runtime `PilotValue` and returns a `bool` while populating a `bindings` dictionary:
1. **Or-Pattern (`p1 | p2` / `p1 or p2`)**: Short-circuits on the first sub-pattern that matches.
2. **And-Pattern (`p1 and p2`)**: Requires all sub-patterns to match (e.g., `> 0 and < 100`).
3. **Not-Pattern (`not p`)**: Succeeds when `p` does not match.
4. **Relational Pattern (`== v`, `!= v`, `< v`, `<= v`, `> v`, `>= v`)**: Compares the subject against `v` using numeric/value comparison.
5. **Type Pattern (`int x`, `str s`, `User u`)**: Checks `MatchesTypeName(value, typeName)` and optionally binds `x` to `value` (or matches a trailing `{ ... }` property pattern).
6. **Guard Clause (`when expr` / `if expr`)**: In `VisitMatch_stmt`, a temporary child scope is pushed, pattern bindings are defined, and the guard expression is evaluated. If the guard evaluates to `false`, the scope is discarded and matching continues to the next `case` arm!

---

## 5. Available Functions, Methods & Properties

| Construct / Operator | Syntax | Description |
| :--- | :--- | :--- |
| Destructuring binding | `let { "a": x = 1, **rest } = map_val` | Unpacks fields with defaults and rest capture; raises `MatchError` on mismatch. |
| Standalone destructuring | `[head, ..tail] = list_val` | Rebinds or defines variables from a list, tuple, or range without `let`/`var`. |
| Guard destructuring | `guard let [a, b] = val else { ... }` | Safely attempts destructuring; runs `else` suite instead of throwing `MatchError`. |
| Inline `is` pattern | `if val is int n { ... }` / `val is not null` | Evaluates any pattern inline as a `bool` expression and binds variables on match. |
| `match` statement | `match val { case ... => ... }` | Evaluates `case` arms in order with optional `when`/`if` guards and `default` fallback. |

---

## 6. Complete Runnable Examples

See [`examples/showcase/08_exhaustive_pattern_matching.pilot`](../../examples/showcase/08_exhaustive_pattern_matching.pilot) for the full executable showcase.

```pilot
// 1. Deep nested record + sequence destructuring with defaults & **rest
let response = {
    "status": 200,
    "data": {
        "user": "grace",
        "scores": [95, 88, 91, 100]
    },
    "region": "us-west",
    "trace_id": "tx-99"
}

let {
    "status": code,
    "data": { "user": username, "scores": [first_score, ..middle_scores, last_score] },
    "tier": tier = "standard",
    **metadata
} = response

assert(code == 200 and username == "grace")
assert(first_score == 95 and middle_scores == [88, 91] and last_score == 100)
assert(tier == "standard")
assert(metadata == {"region": "us-west", "trace_id": "tx-99"})

// 2. Exhaustive 'match' with relational, disjunction, type, and guarded patterns
def classify_event(evt: dynamic) -> str {
    match evt {
        case 200 | 201 | 204 => return "HTTP_SUCCESS"
        case > 400 and < 500 => return "CLIENT_ERROR"
        case {"type": "metric", "value": v} when v >= 90 => return f"HIGH_METRIC:{v}"
        case [head, ..tail] => return f"BATCH(first={head}, rest={len(tail)})"
        case str text => return f"TEXT:{text}"
        default => return "UNKNOWN"
    }
}

assert(classify_event(201) == "HTTP_SUCCESS")
assert(classify_event(404) == "CLIENT_ERROR")
assert(classify_event({"type": "metric", "value": 98}) == "HIGH_METRIC:98")
assert(classify_event([10, 20, 30]) == "BATCH(first=10, rest=2)")
assert(classify_event("ping") == "TEXT:ping")
```

---

[← Prev: Contextual Parameters & DI](./07-contextual-parameters-di.md) | [🏠 Documentation Hub](../index.md) | [🔤 Lexical Reference](../lexical-reference.md) | [📐 Syntax Reference](../syntax-reference.md) | [⚙️ Functions & API](../functions-and-apis.md) | [Next: Design-by-Contract & Stealing →](./09-design-by-contract.md)
