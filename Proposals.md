# Proposals.md: Dynamic Name-Blind Keyword Reflection & User-Extensible Category Engine

## Executive Summary & Problem Framing
This proposal addresses the core architectural mandate:
> *"Update `Proposals.md` and let me know the best course of action if every keyword symbol (excluding the seed keywords) is defined and unknown what their names are, what their descriptions are, and only knowing the category in which it falls under. I believe there should be a lot more categories than just those 6 and possible user-defined categories are possible."*

### The Core Architectural Challenge
Under the **Universal Name-Blind Contract** ([`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md)):
1. **Strict 14-Token Hardware Boundary:** The Rust workspace (`crates/pilot-*`) must never hardcode any keyword beyond the 14 hardware seed tokens:
   `__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`.
2. **Zero Knowledge of Names or Descriptions:** The Rust compiler has **zero prior knowledge** of what any non-seed keyword is named, what it does, or what its hover documentation says.
3. **Category-Only Semantic Physics:** The Rust engine only knows the **category** under which a symbol falls (e.g., Category ID $C$).
4. **Open & User-Extensible Taxonomy:** Categories cannot be restricted to an arbitrary fixed list of 6 items. The language requires a rich taxonomy of core categories (15+), and **user-defined categories must be dynamically registerable** by external packages and DSLs (e.g., GPU shaders, reactive UI frameworks, neural physics operators).

---

## 1. Technical Bottlenecks in the Current Rust Engine

To support this zero-knowledge model, four specific bottlenecks in the Rust crates must be addressed:

### 1.1 `cmd_ide_sync` Isolation in `crates/pilot-cli/src/main.rs`
When running `cargo run -p pilot-cli -- ide-sync`, `cmd_ide_sync()` compiles and executes [`selfhost/ide_sync.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ide_sync.pilot) via a bare `PilVirtualMachine`:
```rust
vm.engine_handler = Some(Box::new(|name, args| {
    if name == "lsp_query_method" {
        Ok(pilot_rt::PilotVal::from_str("ide-sync"))
    } else if name == "fs_write_text" && args.len() >= 2 {
        // ... writes file ...
    } else if name == "fs_read_text" && !args.is_empty() {
        // ... reads file ...
    } else {
        Ok(pilot_rt::PilotVal::NULL)
    }
}));
```
**Bottleneck:** `cmd_ide_sync` only handles `"lsp_query_method"`, `"fs_write_text"`, and `"fs_read_text"`. Any dynamic reflection hooks (`"query_registered_symbols"`, `"ast_node_count"`, `"mount_unit"`, etc.) return `NULL`. `cmd_ide_sync` does not create a `PhysicsRegistry` or delegate unknown hooks to `pilot_analysis::handle_engine_intrinsic`.

### 1.2 Missing Hook in `crates/pilot-analysis/src/infer.rs`
In [`stdlib/core/core.pilot:515`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/core.pilot#L515), Pilot expects:
```pilot
__registered_symbols(category_id) => __engine(Cfg.Hooks.sym_registry, category_id);
```
Where `Cfg.Hooks.sym_registry` is `"query_registered_symbols"`. In `handle_engine_intrinsic` ([`infer.rs:193-361`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/src/infer.rs#L193-L361)), this hook is not handled and returns `PilotVal::NULL`.

### 1.3 `flatten_ast` Drops String Attribute Arguments
In [`infer.rs:86-96`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/src/infer.rs#L86-L96):
```rust
if an.node_kind() == NodeKind::IntLit {
    attr_args.push(arena.i64_pool.get(an.lhs as usize).copied().unwrap_or(0));
    attr_arg_count += 1;
}
```
**Bottleneck:** When a Pilot declaration defines an attribute with hover documentation (e.g. `[Keyword(category: 1, doc: "Iterates over ranges")]`), `NodeKind::StrLit` arguments are dropped entirely, preventing Pilot from retrieving documentation reflectively.

### 1.4 Whitelist Lock in `scripts/gate1_check.py`
`scripts/gate1_check.py` strictly whitelists string literals allowed in `infer.rs`. Any new or restored reflection hook name must be included in `ALLOWED_INFER_STRINGS`.

---

## 2. The Comprehensive Open Category Taxonomy

To replace the legacy 6-category stub, the language and IDE system will use an open, integer-keyed category system partitioned into **Hardware Seed Core**, **Language Standard Categories (1–99)**, and **User-Defined Categories ($\ge 100$)**:

| Category ID | Semantic Domain | TextMate Scope Selector | Hover Badge | Example Pilot Keywords (Defined in Pilot, Never in Rust) |
| :---: | :--- | :--- | :--- | :--- |
| **0** | **Hardware Seed** | `keyword.control.seed.pilot` | `Hardware Seed` | `__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm` |
| **1** | **Control Flow** | `keyword.control.flow.pilot` | `Control Flow` | `while`, `until`, `for`, `in`, `then`, `break`, `continue`, `guard`, `switch`, `case`, `default`, `try`, `catch`, `else` |
| **2** | **Type Declarations** | `keyword.declaration.pilot` | `Declaration` | `var`, `const`, `enum`, `actor`, `state`, `type`, `alias` |
| **3** | **Visibility Modifiers** | `storage.modifier.visibility.pilot` | `Visibility` | `pub`, `priv`, `prot`, `pkg`, `int`, `file`, `open`, `export`, `lvl` |
| **4** | **Storage Modifiers** | `storage.modifier.pilot` | `Modifier` | `pure`, `static`, `iso`, `partial`, `ext` |
| **5** | **Property Accessors** | `keyword.other.property.pilot` | `Property Accessor`| `get`, `set`, `init`, `will_set`, `did_set` |
| **6** | **Contextual Variables**| `variable.language.property.pilot` | `Language Var` | `field`, `value`, `self` |
| **7** | **Lifecycle & RAII** | `keyword.other.lifecycle.pilot` | `Lifecycle` | `scope`, `enter`, `exit` |
| **8** | **Design by Contract** | `keyword.other.contract.pilot` | `Contract` | `requires`, `ensures`, `old`, `invariant` |
| **9** | **Adapters & Context** | `keyword.other.adapter.pilot` | `Adapter / DI` | `context`, `adapter`, `each` |
| **10** | **Bundle Collision** | `keyword.other.collision.pilot` | `Composition` | `coalesce`, `with`, `without`, `as` |
| **11** | **Concurrency & Real-Time**| `keyword.control.concurrency.pilot`| `Concurrency` | `async`, `await`, `spawn`, `atomic`, `sync`, `isolate`, `select`, `timeout`, `bounded`, `noalloc`, `region`, `deadline`, `within`, `periodic`, `priority`, `jitter` |
| **12** | **Database & Query** | `keyword.control.database.pilot` | `Database / LINQ` | `database`, `transaction`, `commit`, `rollback`, `savepoint`, `connect`, `from`, `where`, `order`, `ascending`, `descending`, `insert`, `into`, `values`, `update`, `delete` |
| **13** | **Foreign & System** | `keyword.control.foreign.pilot` | `Foreign ABI` | `extern`, `assembly`, `module`, `lib` |
| **14** | **Metaprogramming** | `keyword.other.query.pilot` | `Metaprogramming` | `sizeof`, `alignof`, `offsetof`, `fields_of`, `nameof`, `assert`, `blueprint`, `cue`, `role`, `route`, `unit`, `syntax`, `op` |
| **$\ge 100$** | **User-Defined Categories** | *Dynamic (User Specified)* | *Dynamic (User)* | *Extensible by third-party packages, GPU kernels, neural frameworks, UI libraries* |

---

## 3. Best Course of Action: The Two-Tier Name-Blind Category Engine

### Tier 1: Declarative Category Registration (Pilot Side)
In Pilot, categories are first-class constructs. Neither their scopes nor badges are hardcoded in Rust.
Pilot code (standard library or user modules) registers categories into the engine via:
```pilot
// In stdlib/core/keywords.pilot or user package:
__engine("register_keyword_category", cat_id, scope_str, hover_badge_str, lsp_kind_u32);
```
Or via attribute definition:
```pilot
[Layout(c_abi)]
pub struct KeywordCategoryDefAttr : IAttribute, IValue {
    pub id:          i32;
    pub scope:       str;
    pub hover_badge: str;
    pub lsp_kind:    u32;
    pub weight:      i32;
}
```

### Tier 2: Keyword Declarations with Attached Metadata (Pilot Side)
All non-seed keywords are declared in Pilot files (such as `stdlib/core/keywords.pilot` or package modules).
Each keyword is declared as a symbol tagged with its category and description:
```pilot
[Keyword(category: 1, doc: "Validates early exit preconditions without nested indentation.")]
struct guard {}

[Keyword(category: 1, doc: "Iterates over a sequence, collection, or range.")]
struct for {}

[Keyword(category: 3, doc: "Public visibility modifier.")]
struct pub {}

[Keyword(category: 11, doc: "Suspends the current coroutine until the awaited task completes.")]
struct await {}

// Example User-Defined Category (GPU Kernel DSL):
[Keyword(category: 101, doc: "Synchronizes all execution threads within the active threadgroup workgroup.")]
struct barrier {}
```

#### What Rust Sees:
- Rust parses `struct guard {}` into an AST node whose symbol is `guard` (or an integer `SymbolId`).
- Rust inspects the attached `[Keyword]` attribute:
  - `category`: `1` (integer)
  - `doc`: `"Validates early exit preconditions without nested indentation."` (string)
- **Rust does not know or care what the word `"guard"` or `"barrier"` means.**
- Rust registers this tuple into the `PhysicsRegistry`:
  $$\text{Keywords}[C] \leftarrow \text{Keywords}[C] \cup \{ (\text{symbol}, \text{doc}) \}$$

---

## 4. How IDE Sync & LSP Consume the Engine Dynamically

### 4.1 Artifact Generation (`selfhost/ide_sync.pilot`)
When generating `vscode-pilot/syntaxes/pilot.tmLanguage.json`:
1. `ide_sync.pilot` queries all registered categories from the engine:
   ```pilot
   let cat_count = __engine("keyword_category_count");
   var i = 0;
   while i < cat_count {
       let cat_id = __engine("keyword_category_at", i);
       let scope  = __engine("keyword_category_scope", cat_id);
       let words  = __engine("query_category_keywords", cat_id); // Pipe-delimited string of symbols
       
       if words != "" {
           // Emits: { "name": scope, "match": "\\b(" + words + ")\\b" }
           add_tm_rule(scope, words);
       }
       i = i + 1;
   }
   ```
2. **Result:** `pilot.tmLanguage.json` is generated with 100% dynamic patterns. Zero non-seed words are hardcoded in `ide_sync.pilot`! Even user-defined categories (e.g., category 101) automatically generate their own TextMate grammar rules!

### 4.2 LSP Hover Dispatch (`crates/pilot-lsp/src/server.rs` & `ide_sync.pilot`)
When a user hovers over any token in VS Code:
1. `ide_sync.pilot` extracts the hovered word from the source slice.
2. It queries the engine:
   ```pilot
   let doc   = __engine("query_keyword_doc", word);
   let badge = __engine("query_keyword_badge", word);
   if doc != "" {
       let md = format_keyword_hover_markdown(word, badge, doc);
       __engine("lsp_set_result", md);
       return;
   }
   ```
3. **Result:** Instant, name-blind hover card display for both standard and user-defined keywords without any hardcoded `switch kw` table!

---

## 5. Concrete Rust Modifications Required

To make this best course of action operational, the following minimal, name-blind changes in Rust are proposed:

### 5.1 In `crates/pilot-analysis/src/infer.rs`
1. Add dynamic storage in `PhysicsRegistry`:
   ```rust
   pub struct CategoryDescriptor {
       pub scope: String,
       pub hover_badge: String,
       pub lsp_kind: u32,
   }

   pub struct KeywordRecord {
       pub sym: String,
       pub doc: String,
   }

   // Inside PhysicsRegistry:
   pub categories: BTreeMap<u32, CategoryDescriptor>,
   pub keywords_by_category: BTreeMap<u32, Vec<KeywordRecord>>,
   pub keyword_index: HashMap<String, (u32, String)>, // word -> (category_id, doc)
   ```
2. In `handle_engine_intrinsic`:
   - `"register_keyword_category"`: Registers $(cat\_id, scope, badge, lsp\_kind)$.
   - `"register_keyword"`: Registers $(sym, cat\_id, doc)$.
   - `"keyword_category_count"`: Returns number of registered categories.
   - `"keyword_category_at"`: Returns category ID at index.
   - `"keyword_category_scope"`: Returns TextMate scope for category.
   - `"query_category_keywords"`: Returns pipe-delimited regex string for category.
   - `"query_keyword_doc"`: Returns hover doc for symbol string.
   - `"query_keyword_badge"`: Returns hover badge for symbol string.
3. Update `flatten_ast` to automatically register nodes decorated with `[Keyword]` into these tables upon mounting.

### 5.2 In `crates/pilot-cli/src/main.rs`
In `cmd_ide_sync()`:
1. Initialize `PhysicsRegistry` and mount `stdlib/core/core.pilot` and `stdlib/core/keywords.pilot`.
2. Update `vm.engine_handler` so unknown hooks delegate directly to `pilot_analysis::handle_engine_intrinsic(reg, op_code, args)`.

### 5.3 In `scripts/gate1_check.py`
Add the new dynamic hooks to `ALLOWED_INFER_STRINGS`:
`"register_keyword_category"`, `"register_keyword"`, `"keyword_category_count"`, `"keyword_category_at"`, `"keyword_category_scope"`, `"query_category_keywords"`, `"query_keyword_doc"`, `"query_keyword_badge"`.

---

## 6. Verification Against Architectural Invariants

| Invariant | Status | Verification Mechanism |
| :--- | :---: | :--- |
| **Strict 14-Token Boundary** | **PASS** | Rust contains 0 hardcoded keyword names or descriptions. |
| **User-Extensible Categories** | **PASS** | Any category ID ($\ge 100$) can be registered dynamically by user libraries. |
| **Line Caps** | **PASS** | `infer.rs` +20 lines ($\le 440$), `main.rs` +15 lines ($\le 520$). |
| **Phase 1A/2B/3B Gates** | **PASS** | Whitelist updated in `gate1_check.py`; zero banned tokens. |
| **Universal Rename Litmus Test**| **PASS** | If `guard` is renamed to `ensure_or_exit` in Pilot, everything compiles and highlights with 0 Rust edits. |

---

## 7. Resolution & Status: Proposal B Approved and Fully Implemented

1. **Permission Granted & Scope:** The user approved Proposal B ("AST Attribute Reflection & Mount-Driven Topology") with permission to modify `infer.rs` and `main.rs`.
2. **Rust Engine Modifications Completed:**
   - `crates/pilot-analysis/src/infer.rs`:
     - Preserved attribute string literal and identifier arguments during AST flattening.
     - Added support in `mount_unit` to offset and append attribute arguments to the active physics registry.
     - Maintained strict line cap: 436 lines ($\le 440$).
   - `crates/pilot-cli/src/main.rs`:
     - Updated `cmd_ide_sync` to instantiate a `PhysicsRegistry` from `selfhost/ide_sync.pilot` and route engine intrinsics directly to `handle_engine_intrinsic`.
     - Maintained strict line cap: 481 lines ($\le 520$).
3. **Pilot-Hosted Declarative Catalog:**
   - `stdlib/core/keywords.pilot`:
     - Implemented `KeywordCategoryDefAttr` with `(id: i32, scope: str, hover_badge: str, lsp_kind: i32)`.
     - Implemented `KeywordDefAttr` with `(category: i32, doc: str)` and `with_suffix`.
     - Declared 15 category descriptors (0..14) and all non-seed keywords decorated with category and documentation attributes.
     - Zero hardcoded non-seed keyword strings in Rust.
4. **Self-Hosted IDE Sync:**
   - `selfhost/ide_sync.pilot`:
     - Implemented dynamic category reflection (`reflect_category_keywords`), doc reflection (`reflect_keyword_doc`), badge reflection (`reflect_keyword_badge`), and related keyword reflection (`reflect_related_keywords`).
     - Avoided runtime struct member expression evaluation in the Pilot VM by returning scalar strings directly.
     - Dynamically generated `vscode-pilot/syntaxes/pilot.tmLanguage.json` with all 15 categories properly scoped.
     - Maintained zero hardcoded non-seed keyword strings across `ide_sync.pilot`.
5. **Verification & Gates:**
   - All 7 Phase 6 LSP tests in `pilot-lsp` pass:
     - `test_pilot_hosted_ide_sync_hover_completion_and_definition` (PASS)
     - `test_name_blind_lsp_rename_litmus_gate` (PASS)
     - `test_selfhost_ide_sync_pilot_generates_all_4_vscode_artifacts_and_hover_badges` (PASS)
     - `test_distributive_headers_variation_b_field_value_and_uncalled_method_groups` (PASS)
     - `test_patch_attribute_cascades_offset_lens_and_custom_ide_token_attr` (PASS)
     - `test_pilot_ide_context_policy_disambiguates_constructors_functions_and_callables` (PASS)
     - `test_live_user_syntax_evolution_and_quote_splice_depth_hygiene` (PASS)
   - CLI tests in `pilot-cli` pass (3/3).
   - `python3 scripts/gate1_check.py && python3 scripts/gate2_check.py && python3 scripts/gate3_check.py` all pass cleanly.
   - `cargo run -p pilot-cli -- ide-sync` emits valid JSON for all 4 artifacts.

---

## 8. Return to Strict Read-Only Mandate for Rust Files

As mandated by the user:
> *"after you are done wiring up the proposal B logic you are to revert back to needing permission for all rust files"*

**Status:** The Rust workspace (`crates/**/*.rs`) has immediately and strictly reverted to **READ-ONLY**. No further modifications to Rust code will occur without explicit, separate user authorization via this document.

---

## 9. Resolution: LSP Hover & Reference Navigation Architecture

### 9.1 Root Causes Identified
1. **Parser Span Truncation for `MemberExpr` (NodeKind 17):**
   - In `pilot-parser`, the span of `MemberExpr` (`acct.balance`) only enclosed the LHS receiver (`acct`, e.g. `186..190`), omitting the dot and member name (`.balance`, e.g. `190..198`).
   - Hover and definition queries on `.balance` or `.deposit` failed span containment checks (`offset <= e`), returning `null`.
2. **Missing Target Resolution for Members & Identifiers:**
   - `MemberExpr` nodes returned `target_id == u32::MAX`, causing fallback to `target_id = best_nid` (the member access itself rather than the declaration).
   - "Go to Definition" on `acct.balance` jumped to line 10 itself, rather than line 4 (`pub balance: i64;`).
3. **Absence of Reference Navigation (`textDocument/references`):**
   - `vscode-pilot/src/extension.ts` registered hover, definition, and document symbol providers, but lacked a registered `ReferenceProvider`.
   - `crates/pilot-lsp/src/server.rs` returned `JsonValue::Null` for unhandled methods.

### 9.2 Pilot-First & Extension Resolution (0 Rust Source Changes)
1. **Dynamic Member Token Span Synthesis (`selfhost/ide_sync.pilot`):**
   - For `NodeKind == 17` (`MemberExpr`), `ide_sync.pilot` scans forward from the receiver's end `e` to find `.` and extracts the precise identifier token bounds `eff_s..eff_e`.
   - Cursor positions within member accesses match with 100% precision.
2. **Semantic Target Declaration Resolution (`selfhost/ide_sync.pilot`):**
   - For `MemberExpr`, the engine searches the AST for matching `FieldDecl` (kind 21) or `FnDecl` (kind 3) nodes and sets `target_id`.
   - Hover on `acct.balance` displays ```pilot\nbalance: i64\n``` with layout size (`8 bytes`).
   - Hover on `acct.deposit(10)` displays ```pilot\npub fn deposit(amount: i64) -> i64\n```.
   - "Go to Definition" on `acct.balance` returns `4:8..4:21` (jumps directly to `pub balance: i64;`).
   - "Go to Definition" on `acct.deposit(10)` returns `5:11..5:18` (jumps directly to `pub fn deposit`).
   - "Go to Definition" on `acct` returns `9:0..9:20` (jumps directly to `let acct = Account();`).
   - "Go to Definition" on `Account` returns `3:7..8:1` (jumps directly to `struct Account { ... }`).
3. **Reference Navigation Provider (`vscode-pilot/src/extension.ts`):**
   - Registered `vscode.languages.registerReferenceProvider(selector, { ... })`.
   - Supports 3-tier resolution:
     1. Live LSP `textDocument/references` query if supported by the server.
     2. Live LSP rename analysis (verifies non-keyword identifier legality and retrieves exact file ranges).
     3. Whole-project multi-file workspace scanner finding all symbol references across all workspace `.pilot` files.
4. **`handle_lsp_references` in `selfhost/ide_sync.pilot`:**
   - Implemented name-blind symbol reference collector finding all matching AST node spans and returning JSON ranges `[{"start": s, "end": e}, ...]`.
   - Wired to top-level dispatcher for `_lsp_method == "textDocument/references"`.
5. **Verification & Invariant Preservation:**
   - All 7 Phase 6 LSP tests in `crates/pilot-lsp` pass (7/7).
   - CLI tests in `pilot-cli` pass (3/3).
   - All Gate checks (`gate1_check.py`, `gate2_check.py`, `gate3_check.py`) pass.
   - `crates/**/*.rs` is completely clean with 0 edits (`git diff crates/` is empty).
   - `npm run compile` in `vscode-pilot` succeeds with 0 errors.

---

## 10. Optional Proposal D: Wire `textDocument/references` Directly into `crates/pilot-lsp/src/server.rs`

*Note: The Rust workspace is strictly read-only. This change will NOT be made without user permission.*

- **File to Edit:** `crates/pilot-lsp/src/server.rs` (+15 lines).
- **Proposed Changes:**
  1. In `initialize` (line 74): Add `"referencesProvider": true` to `capabilities`.
  2. In `dispatch_to_pilot_ide_sync` (around line 306): Add `match method` arm for `"textDocument/references"` to parse the JSON array emitted by `selfhost/ide_sync.pilot` and convert `s:e` offsets to LSP `Location[]` objects.
- **Benefit:** Allows third-party CLI, Neovim, Helix, and Sublime LSP clients connecting over raw stdio JSON-RPC to receive `textDocument/references` directly from the standalone server executable.
