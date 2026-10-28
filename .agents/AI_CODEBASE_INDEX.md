# Pilot — AI-Only Codebase Navigation & Symbol Index

> **STRICTLY FOR AI AGENTS**: Do not link this file in user-facing documentation. Use exact paths and `:L<line>` anchors below with `read_file` (`StartLine`/`EndLine`) to jump directly to code without exploratory directory/grep bouncing.
> **Auto-Sync Stamp**: `7A6087F5E3CE9CCFD40173DB67020191854A3BE3725659C24AFE3722A3A5B5DA` | **Total Indexed Files**: 301 | **Regenerate**: `pwsh -NoProfile -File .agents/scripts/update_ai_index.ps1`

<!-- BEGIN MANUAL NOTES -->
## 1. Instant Task-to-File Routing Table (Consult First — Zero Blind Searching)

| Subsystem / Task | Primary Entry Files | Key Notes |
| :--- | :--- | :--- |
| **Roadmap, Concurrency & Invariants** | `MILESTONES.md` | Single source of truth for M1–M27 status, parallel waves, and grammar invariants. |
| **Master Lexer & Parser Grammar** | `antlr/Pilot.g4`, `Compiler/PilotParserFast.cs` | ANTLR4 combined grammar (generated C# parser goes to `obj/` on `dotnet build`). |
| **Preprocessor (`#define`, `#if`, `#pragma`)** | `Preprocessing/PilotPreprocessor.cs` | Runs before lexing; emits `PREPROC001`–`PREPROC011`. |
| **Tree-Walk AST Interpreter** | `Runtime/Execution/PilotInterpreterVisitor.cs`, `Runtime/Execution/PilotEngine.cs` | Evaluates statements/expressions in AST mode; pattern matching in `PilotPatternMatcher.cs`. |
| **Runtime Values & Primitives** | `Runtime/Values/PilotValue.cs`, `PilotValueKind.cs`, `PilotAdt.cs`, `PilotObject.cs` | Also `PilotFunction.cs`, `PilotMethodGroup.cs`, `PilotKeyword.cs`, `PilotMatrix.cs`, `PilotRange.cs`, `PilotGenerator.cs`, `PilotLazy.cs`. |
| **Bytecode Compiler & `.pilc` Format** | `Compiler/PilotBytecodeCompiler.cs`, `Compiler/PilotCompiledProgram.cs`, `Runtime/Bytecode/PilotOpcode.cs` | Two-phase AST -> PilVM chunk compiler and `.pilc` v2 serializer. |
| **PilVM Virtual Machine** | `Runtime/Bytecode/PilVirtualMachine.cs`, `Runtime/Bytecode/PilotBytecodeChunk.cs` | Stack/register bytecode VM execution loop. |
| **Static Analysis & Type Inference** | `Analysis/PilotAnalysisPipeline.cs`, `Analysis/TypeSystem/PilotTypeInferenceEngine.cs`, `PilotType.cs` | Orchestrates inference, exhaustiveness, borrow checking, dimensions, and contracts. |
| **Exhaustive Pattern Matching** | `Analysis/PatternMatching/PilotExhaustivenessChecker.cs`, `Runtime/Execution/PilotPatternMatcher.cs` | Compile-time usefulness/exhaustiveness checking + runtime destructuring. |
| **Ownership, Loops, Units & Contracts** | `Analysis/Ownership/PilotBorrowChecker.cs`, `Analysis/Loops/PilotForLoopAnalyzer.cs`, `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs`, `Analysis/ContractVerification/PilotContractVerifier.cs` | Runtime contracts registry in `Runtime/Contracts/PilotContractRegistry.cs`; loop ABI types in `Runtime/NativeMemory/PilotLoopAbiTypes.cs`. |
| **Built-in Functions & Host Scripting** | `Runtime/Builtins/PilotBuiltinRegistry.cs`, `Scripting/PilotScriptContext.cs`, `Scripting/PilotScript.cs` | Global intrinsics (`println`, `len`, `json`, `venv`, `matrix`, file/OS helpers). |
| **Type Adapters & Extensions** | `Runtime/Adapters/PilotTypeAdapterRegistry.cs`, `Runtime/Extensions/PilotLexicalExtension.cs` | First-class `adapter` conversion chains & `extension ... on ...` dispatch. |
| **Compile-Time `@` Code Generators** | `CodeGen/Engine/PilotCodeGeneratorPipeline.cs`, `CodeGen/Builtins/*.cs` | `@builder`, `@equality`, `@json`, `@metadata`, `@observable`, `@repr`. |
| **Bidirectional Source Transpilers** | `Transpilation/TranspilationRegistry.cs`, `PilotLibraryTranspiler.cs`, `PilotLibraryMapper.cs`, `Transpilation/Targets/*` | `--emit <target>` and `--from <lang>` for C, C++, C#, Rust, Python, JS, Lua + `PilotOptimizer.cs`. |
| **Native AOT (LLVM, WASM, C-ABI, ASM)** | `Native/PilotNativeCompiler.cs`, `Native/Llvm/PilotLlvmEmitter.cs`, `Native/Wasm/PilotWasmEmitter.cs`, `Native/CAbi/PilotCAbiEmitter.cs`, `Native/Assembly/PilotAssemblyEmitter.cs` | Native C header in `Native/Runtime/pilot_runtime.h`; Burst/Zero-GC in `Runtime/Burst/PilotBurstEngine.cs`, `Runtime/NativeMemory/`. |
| **Polyglot Interop Blocks** | `Runtime/Interop/PolyglotRegistry.cs`, `Runtime/Interop/PolyglotTypeBridge.cs` | Inline Python, JS, Java, Lua, C, C++, ASM, C#, Rust, PostgreSQL, MySQL + yield bridges. |
| **Packages (`pkg`) & Virtual Envs (`venv`)** | `Runtime/Packaging/PilotPackageManager.cs`, `Runtime/Packaging/Adapters/BaseInteropPackageAdapter.cs`, `Runtime/Venv/PilotVirtualEnvironment.cs` | `.pilpkg`, `pilot.json`, `pilot.lock.json`, foreign package adoption, `pilotvenv.cfg`. |
| **Debugger & Hot Reload** | `Debugging/PilotDebugger.cs`, `Debugging/PilotCliDebugger.cs`, `Runtime/HotReload/PilotHotReloader.cs` | Breakpoints, stepping, call stack, state-preserving hot reload. |
| **LSP Server & Semantic Diagnostics** | `Pilot.LanguageServer/Lsp/PilotLspServer.cs`, `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs`, `PilotDiagnosticCatalog.cs`, `PilotSymbol.cs` | Hover, completion, semantic tokens, go-to-def, code actions, quick fixes. |
| **VS Code Extension** | `vscode-pilot/src/extension.ts`, `vscode-pilot/syntaxes/pilot.tmLanguage.json`, `vscode-pilot/package.json` | LSP client, Run/Debug editor buttons, `launch.json` provider, TextMate grammar. |
| **CLI Driver & Interactive REPL** | `Cli/Program.cs`, `Cli/PilotRepl.cs` | All `pilot` subcommands and flags. |
| **Automated Tests** | `Pilot.Tests/*.cs`, `tests/*.pilot`, `Testing/PilotTestRunner.cs` | Run `dotnet test --filter FullyQualifiedName~<TestClass>` for fast targeted verification. |

### Cross-Layer Checklist When Adding/Modifying a Language Feature
1. **Grammar**: `antlr/Pilot.g4` (check `MILESTONES.md` Conflict Resolutions first; run `dotnet build Pilot.csproj` to regenerate ANTLR parser).
2. **Static Analysis**: `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (+ `PilotExhaustivenessChecker.cs` / `PilotBorrowChecker.cs` if applicable).
3. **AST Execution**: `Runtime/Execution/PilotInterpreterVisitor.cs` (+ `Runtime/Values/PilotValue.cs` if new value representation).
4. **Bytecode & VM**: `Runtime/Bytecode/PilotOpcode.cs` -> `Compiler/PilotBytecodeCompiler.cs` -> `Runtime/Bytecode/PilVirtualMachine.cs` (+ `PilotCompiledProgram.cs` if serialized).
5. **IDE / LSP**: `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` + `PilotDiagnosticCatalog.cs` (+ `vscode-pilot/syntaxes/pilot.tmLanguage.json` for highlighting).
6. **Codegen / Emitters (if applicable)**: `Native/*Emitter.cs` and `Transpilation/Targets/*TargetConverter.cs`.
7. **Tests**: Add xUnit tests in `Pilot.Tests/` and `.pilot` script in `tests/`.
<!-- END MANUAL NOTES -->

---

## 2. Master Grammar Map (`antlr/Pilot.g4` Exact Line Anchors)

Total lines in `antlr/Pilot.g4`: **1690**

### Parser Rules (236)
`program`:L15 · `identifier`:L23 · `stmt`:L53 · `simple_stmt`:L58 · `small_stmt`:L62 · `compound_stmt`:L90
`suite`:L141 · `guard_stmt`:L151 · `pattern_guard`:L155 · `defer_stmt`:L160 · `type_annotation`:L168
`return_type`:L172 · `type_spec`:L176 · `error_union_type`:L180 · `error_primary_type`:L184 · `union_type`:L191
`intersection_type`:L195 · `primary_type`:L199 · `base_type`:L203 · `type_arguments`:L213 · `generic_params`:L217
`generic_param`:L221 · `where_type_clause`:L226 · `where_type_item`:L230 · `access_modifier`:L235
`declaration_modifier`:L242 · `binding_pattern`:L252 · `tuple_pattern`:L260 · `list_pattern`:L264
`record_pattern`:L268 · `binding_pattern_item`:L272 · `record_pattern_field`:L276 · `pattern_assign_stmt`:L281
`let_stmt`:L285 · `var_stmt`:L289 · `const_stmt`:L293 · `ref_stmt`:L297 · `dynamic_stmt`:L301
`type_alias_stmt`:L305 · `adt_sum_body`:L310 · `adt_variant`:L316 · `adt_variant_payload`:L320
`adt_field_list`:L324 · `adt_field`:L328 · `annassign_stmt`:L333 · `expr_stmt`:L337 · `channel_send_stmt`:L341
`augassign`:L345 · `func_def`:L365 · `contract_clause`:L369 · `parameters`:L374 · `parameter`:L378
`class_def`:L382 · `state_def`:L387 · `class_bases`:L391 · `struct_def`:L396 · `record_def`:L401
`struct_member`:L406 · `interface_def`:L411 · `enum_def`:L416 · `enum_suite`:L420 · `enum_member`:L424
`actor_def`:L429 · `model_def`:L434 · `property_def`:L443 · `property_body`:L447 · `property_accessor`:L451
`get_accessor`:L456 · `set_accessor`:L460 · `operator_def`:L465 · `overloadable_op`:L469 · `extension_def`:L477
`adapter_def`:L483 · `macro_def`:L488 · `quote_stmt`:L492 · `quote_expr`:L496 · `quasiquote_expr`:L500
`splice_expr`:L504 · `extern_def`:L511 · `extern_member`:L515 · `foreign_language`:L521
`inline_foreign_block`:L525 · `decorators`:L530 · `decorator`:L534 · `labeled_stmt`:L542 · `block`:L546
`if_stmt`:L550 · `while_stmt`:L554 · `for_stmt`:L558 · `for_target`:L562 · `match_stmt`:L567 · `match_suite`:L572
`match_arm`:L576 · `pattern`:L580 · `or_pattern`:L584 · `and_pattern`:L588 · `not_pattern`:L592
`primary_pattern`:L598 · `range_pattern`:L611 · `range_bound`:L615 · `signed_number_pattern`:L620
`keyword_pattern`:L624 · `pattern_arg`:L628 · `relational_pattern`:L633 · `collection_pattern`:L637
`slice_pattern_element`:L641 · `property_pattern`:L646 · `property_pattern_member`:L650
`tuple_pattern_match`:L654 · `type_or_var_pattern`:L658 · `generic_pattern_suffix`:L663 · `try_stmt`:L667
`except_clause`:L671 · `with_stmt`:L675 · `with_item`:L679 · `with_context_binding`:L686 · `flow_stmt`:L690
`pass_stmt`:L698 · `assert_stmt`:L702 · `steal_stmt`:L706 · `delegate_stmt`:L710 · `contract_forward_stmt`:L714
`global_stmt`:L718 · `import_stmt`:L722 · `foreign_import_stmt`:L728 · `foreign_target`:L733 · `import_name`:L737
`import_from`:L741 · `import_as_name`:L745 · `dotted_as_name`:L749 · `import_as_names`:L753
`dotted_as_names`:L757 · `dotted_name`:L761 · `deadline_stmt`:L769 · `within_stmt`:L773 · `periodic_stmt`:L777
`noalloc_stmt`:L781 · `region_stmt`:L785 · `duration_term`:L789 · `spawn_stmt`:L798 · `atomic_stmt`:L802
`sync_stmt`:L806 · `isolate_stmt`:L810 · `select_stmt`:L814 · `select_suite`:L818 · `select_arm`:L822
`select_case_target`:L828 · `dsl_stmt`:L838 · `dsl_args`:L842 · `unit_stmt`:L846 · `unit_expr`:L850
`unit_term`:L854 · `database_stmt`:L865 · `database_def`:L869 · `transaction_stmt`:L874 · `savepoint_stmt`:L878
`savepoint_block`:L882 · `commit_stmt`:L886 · `rollback_stmt`:L890 · `insert_stmt`:L895
`insert_values_group`:L902 · `on_conflict_clause`:L906 · `update_stmt`:L911 · `set_assignment_list`:L918
`set_assignment`:L922 · `delete_stmt`:L927 · `returning_clause`:L933 · `index_clause`:L938 · `index_field`:L942
`index_type`:L946 · `index_hint`:L950 · `query_expr`:L955 · `from_clause`:L959 · `join_clause`:L963
`where_clause`:L967 · `group_clause`:L971 · `order_clause`:L975 · `ordering_term`:L979 · `select_clause`:L983
`select_projection`:L987 · `select_item`:L992 · `limit_clause`:L996 · `sql_query_expr`:L1001 · `from_source`:L1005
`lookup_expr`:L1010 · `set_op`:L1015 · `testlist`:L1026 · `exprlist`:L1030 · `test`:L1034 · `lambda_expr`:L1039
`lambda_body`:L1048 · `capture_list`:L1053 · `capture_item`:L1057 · `lambda_params`:L1063 · `lambda_param`:L1068
`pipe_expr`:L1073 · `set_query_expr`:L1078 · `coalesce_expr`:L1085 · `catch_clause`:L1089 · `or_test`:L1095
`and_test`:L1099 · `not_test`:L1103 · `comparison`:L1108 · `comp_op`:L1112 · `range_expr`:L1125
`bitor_expr`:L1129 · `bitxor_expr`:L1133 · `bitand_expr`:L1137 · `shift_expr`:L1141 · `arith_expr`:L1145
`term`:L1149 · `factor`:L1153 · `unary_op`:L1159 · `power`:L1169 · `atom_expr`:L1173 · `trailer`:L1177
`cascade_trailer`:L1194 · `cascade_target`:L1198 · `over_clause`:L1205 · `subscriptlist`:L1209 · `subscript`:L1213
`atom`:L1221 · `collection_expr`:L1249 · `collection_element`:L1253 · `nameof_expr`:L1257 · `macro_call`:L1261
`symbol_literal`:L1265 · `literal`:L1270 · `testlist_comp`:L1289 · `dictorsetmaker`:L1293 · `dict_entry`:L1298
`comp_for`:L1303 · `comp_iter`:L1307 · `comp_if`:L1312 · `arglist`:L1316 · `argument`:L1320

### Lexer Tokens (256)
`PUBLIC`:L1333 · `PRIVATE`:L1334 · `PROTECTED`:L1335 · `INTERNAL`:L1336 · `STATIC`:L1337 · `PARTIAL`:L1338
`STEAL`:L1339 · `CONTRACTS`:L1340 · `DELEGATE`:L1341 · `PARAMS`:L1342 · `ARGS`:L1343 · `KWARGS`:L1344
`NAMEOF`:L1345 · `EACH`:L1346 · `REPEAT`:L1347 · `ADAPTER`:L1348 · `CONVERTER`:L1349 · `DEF`:L1351 · `FN`:L1352
`CLASS`:L1353 · `STRUCT`:L1354 · `RECORD`:L1355 · `INTERFACE`:L1356 · `PROTOCOL`:L1357 · `ENUM`:L1358
`ACTOR`:L1359 · `LET`:L1360 · `VAR`:L1361 · `CONST`:L1362 · `TYPE`:L1363 · `REF`:L1364 · `DYNAMIC`:L1365
`SYMBOL`:L1366 · `PURE`:L1367 · `MUT`:L1368 · `IMMUT`:L1369 · `PROPERTY`:L1370 · `GET`:L1371 · `OPERATOR`:L1372
`EXTENSION`:L1373 · `EXTEND`:L1374 · `STATE`:L1375 · `SELECT`:L1378 · `FROM`:L1379 · `WHERE`:L1380 · `JOIN`:L1381
`INNER`:L1382 · `LEFT`:L1383 · `RIGHT`:L1384 · `FULL`:L1385 · `CROSS`:L1386 · `OUTER`:L1387 · `ON`:L1388
`GROUP`:L1389 · `BY`:L1390 · `HAVING`:L1391 · `ORDER`:L1392 · `ASC`:L1393 · `DESC`:L1394 · `ASCENDING`:L1395
`DESCENDING`:L1396 · `LIMIT`:L1397 · `OFFSET`:L1398 · `INTO`:L1399 · `DISTINCT`:L1400 · `UNION`:L1401 · `ALL`:L1402
`INTERSECT`:L1403 · `EXCEPT`:L1404 · `DIFFERENCE`:L1405 · `INDEX`:L1406 · `INDEXED`:L1407 · `LOOKUP`:L1408
`TABLE`:L1409 · `SCHEMA`:L1410 · `INSERT`:L1411 · `UPDATE`:L1412 · `DELETE`:L1413 · `VALUES`:L1414 · `SET_KW`:L1415
`RETURNING`:L1416 · `CONFLICT`:L1417 · `DO`:L1418 · `NOTHING`:L1419 · `OVER`:L1420 · `PARTITION`:L1421
`WINDOW`:L1422 · `TRANSACTION`:L1423 · `COMMIT`:L1424 · `ROLLBACK`:L1425 · `SAVEPOINT`:L1426 · `TO`:L1427
`CONNECT`:L1428 · `DATABASE`:L1429 · `MODEL`:L1430 · `ASYNC`:L1433 · `AWAIT`:L1434 · `SPAWN`:L1435
`PARALLEL`:L1436 · `CHAN`:L1437 · `CHANNEL`:L1438 · `ATOMIC`:L1439 · `SYNC`:L1440 · `LOCK`:L1441 · `ISOLATE`:L1442
`REALTIME`:L1445 · `DEADLINE`:L1446 · `ON_MISSED`:L1447 · `WITHIN`:L1448 · `FALLBACK`:L1449 · `PERIOD`:L1450
`PERIODIC`:L1451 · `PRIORITY`:L1452 · `JITTER`:L1453 · `BOUNDED`:L1454 · `NOALLOC`:L1455 · `STACKALLOC`:L1456
`REGION`:L1457 · `WCET`:L1458 · `HARD_RT`:L1459 · `SOFT_RT`:L1460 · `EXTERN`:L1463 · `CSHARP`:L1464
`PYTHON`:L1465 · `RUST`:L1466 · `JS`:L1467 · `JAVASCRIPT`:L1468 · `JAVA`:L1469 · `ASM`:L1470 · `ASSEMBLY`:L1471
`CPP`:L1472 · `C_KW`:L1473 · `LUA`:L1474 · `SQL_KW`:L1475 · `POSTGRES`:L1476 · `POSTGRESQL`:L1477 · `MYSQL`:L1478
`MODULE`:L1479 · `LIB`:L1480 · `DSL`:L1483 · `DOMAIN`:L1484 · `UNIT`:L1485 · `MACRO`:L1486 · `DEFMACRO`:L1487
`QUOTE`:L1488 · `QUASIQUOTE`:L1489 · `SPLICE`:L1490 · `UNQUOTE`:L1491 · `UNQUOTE_SPLICING`:L1492 · `MATCH`:L1493
`CASE`:L1494 · `WHEN`:L1495 · `DEFAULT`:L1496 · `TIMEOUT`:L1497 · `GUARD`:L1500 · `DEFER`:L1501 · `CONTEXT`:L1502
`REQUIRES`:L1503 · `ENSURES`:L1504 · `IF`:L1505 · `ELIF`:L1506 · `ELSE`:L1507 · `THEN`:L1508 · `WHILE`:L1509
`FOR`:L1510 · `IN`:L1511 · `RETURN`:L1512 · `YIELD`:L1513 · `BREAK`:L1514 · `CONTINUE`:L1515 · `PASS`:L1516
`RAISE`:L1517 · `TRY`:L1518 · `CATCH`:L1519 · `FINALLY`:L1520 · `WITH`:L1521 · `AS`:L1522 · `ASSERT`:L1523
`IMPORT`:L1524 · `GLOBAL`:L1525 · `LAMBDA`:L1526 · `AND`:L1527 · `OR`:L1528 · `NOT`:L1529 · `IS`:L1530
`TRUE`:L1533 · `FALSE`:L1534 · `SOME`:L1535 · `NONE`:L1536 · `NULL`:L1537 · `UNDEFINED`:L1538
`JSON_GET_TEXT`:L1541 · `ARROW_LEFT`:L1542 · `ARROW`:L1543 · `FAT_ARROW`:L1544 · `PIPE_FORWARD`:L1545
`PIPE_BACKWARD`:L1546 · `SAFE_CASCADE`:L1547 · `SAFE_NAV`:L1548 · `COALESCE_EQUAL`:L1549 · `COALESCE`:L1550
`JSON_CONTAINS`:L1551 · `JSON_CONTAINED`:L1552 · `ELLIPSIS`:L1553 · `DOTDOT_EQUAL`:L1554 · `DOTDOT_LESS`:L1555
`DOTDOT`:L1556 · `DOUBLESTAR`:L1557 · `DOUBLESLASH`:L1558 · `EQEQUAL`:L1559 · `NOTEQUAL`:L1560 · `LESSEQUAL`:L1561
`GREATEREQUAL`:L1562 · `LEFTSHIFT`:L1563 · `RIGHTSHIFT`:L1564 · `PLUSEQUAL`:L1565 · `MINEQUAL`:L1566
`STAREQUAL`:L1567 · `SLASHEQUAL`:L1568 · `PERCENTEQUAL`:L1569 · `AMPEREQUAL`:L1570 · `VBAREQUAL`:L1571
`CIRCUMFLEXEQUAL`:L1572 · `LEFTSHIFTEQUAL`:L1573 · `RIGHTSHIFTEQUAL`:L1574 · `DOUBLESTAREQUAL`:L1575
`DOUBLESLASHEQUAL`:L1576 · `WALRUS`:L1577 · `LPAR`:L1580 · `RPAR`:L1581 · `LSQB`:L1582 · `RSQB`:L1583
`LBRACE`:L1584 · `RBRACE`:L1585 · `COLON`:L1586 · `SEMI`:L1587 · `COMMA`:L1588 · `DOT`:L1589 · `AT`:L1590
`PLUS`:L1591 · `MINUS`:L1592 · `STAR`:L1593 · `SLASH`:L1594 · `PERCENT`:L1595 · `VBAR`:L1596 · `AMPER`:L1597
`CIRCUMFLEX`:L1598 · `TILDE`:L1599 · `LESS`:L1600 · `GREATER`:L1601 · `EQUAL`:L1602 · `QUESTION`:L1603
`EXCLAMATION`:L1604 · `DOLLAR`:L1605 · `BACKQUOTE`:L1606 · `UNDERSCORE`:L1607

---

## 3. Large File Internal Method Maps (Files > 350 Lines)

Use these `:L<line>` method anchors to pass exact `StartLine`/`EndLine` into `read_file` for large files:

### `Analysis/Loops/PilotForLoopAnalyzer.cs` (473 lines)
`CheckProgram`:L66 · `VisitFor_stmt`:L71 · `AnalyzeSingleLoop`:L81 · `IsInExpressionPosition`:L202
`IsInsideCoroutineGeneratorFunction`:L226 · `FunctionHasValuedReturn`:L264 · `CollectEnclosingLabels`:L280
`ScanLoopBodyTopLevel`:L299 · `ScanSimpleStmt`:L323 · `ScanTree`:L345 · `InspectFlowStmt`:L377

### `Analysis/Ownership/PilotBorrowChecker.cs` (503 lines)
`PushScope`:L84 · `PopScope`:L89 · `CheckRootScopeExit`:L112 · `RegisterResource`:L134 · `TryLookupResource`:L143
`VisitProgram`:L157 · `VisitStruct_def`:L171 · `VisitClass_def`:L188 · `VisitLet_stmt`:L205 · `VisitVar_stmt`:L245
`VisitFunc_def`:L284 · `VisitExpr_stmt`:L308 · `CheckRhsForMove`:L339 · `CheckExpressionUsage`:L435
`CheckZeroAllocation`:L471 · `GetExactText`:L487 · `HasDecorator`:L498

### `Analysis/PatternMatching/PilotExhaustivenessChecker.cs` (2046 lines)
`CheckProgram`:L46 · `CollectDeclarations`:L63 · `RegisterTypeAliasAdt`:L98 · `RegisterEnumAdt`:L139
`RegisterRecord`:L154 · `RegisterStructAsRecord`:L162 · `RegisterClassTypestates`:L171 · `StoreAdt`:L189
`RefreshLayouts`:L198 · `ExtractGenericParams`:L206 · `ExtractFields`:L209 · `VisitFunc_def`:L240
`VisitLet_stmt`:L267 · `VisitVar_stmt`:L279 · `VisitMatch_stmt`:L291 · `VisitCoalesce_expr`:L297
`InferExpressionTextType`:L332 · `ResolveDeclType`:L351 · `InferScrutineeType`:L360 · `TryExtractAtom`:L439
`ParseRichType`:L448 · `OrPat`:L541 · `CtorPat`:L542 · `CtorDesc`:L563 · `VerifyMatch`:L571
`RefineTypeFromPatterns`:L668 · `TryInferTypeFromSinglePat`:L679 · `TryEvaluateStaticBoolGuard`:L717
`LowerPattern`:L726 · `LowerOrPattern`:L732 · `LowerAndPattern`:L739 · `LowerNotPattern`:L765
`NegatePattern`:L782 · `LowerPrimaryPattern`:L821 · `LowerRangePattern`:L888 · `ParseBoundInt`:L916
`ParseUnsignedLong`:L922 · `LowerLiteralPattern`:L931 · `LowerRelationalPattern`:L957
`LowerCollectionPattern`:L973 · `LowerPropertyPattern`:L1015 · `LowerTypeOrVarPattern`:L1040
`SubstituteGenericType`:L1249 · `MakeIntInterval`:L1268 · `GetIntDomainBounds`:L1271 · `IsUseful`:L1289
`FindMissingWitnesses`:L1387 · `FindWitnessVector`:L1420 · `CheckSignatureCompleteness`:L1482
`GetFullClosedDomainConstructors`:L1496 · `SpecializeMatrix`:L1665 · `DefaultMatrix`:L1769
`ExpandColumnZeroOrs`:L1782 · `ExpandOrRow`:L1799 · `PartitionInterval`:L1817 · `ComputeListSplitBound`:L1847
`ExpandSliceRowArgs`:L1863 · `ExpandSliceQueryArgs`:L1878 · `GetCtorSubTypes`:L1881 · `GetListElementType`:L1920
`FormatConstructorWitness`:L1925 · `FormatIntWitness`:L1947 · `FormatInterval`:L1957 · `PrependWildcards`:L1968
`ConcatPatterns`:L1977 · `ConcatTypes`:L1986 · `PrependRepeated`:L1995 · `IsIntegerType`:L2004
`IsBuiltinTypeName`:L2007 · `StripTypeArgs`:L2013 · `StripQuotes`:L2019 · `SplitTopLevel`:L2022

### `Analysis/TypeSystem/PilotErrorFlowAnalyzer.cs` (1240 lines)
`ResolveVariable`:L64 · `CheckProgram`:L88 · `PushScope`:L95 · `PopScope`:L96 · `PrescanDeclarations`:L104
`RegisterTypeAlias`:L134 · `RegisterEnum`:L179 · `RegisterAdapter`:L195 · `RegisterFunction`:L205
`ResolveInferredErrorSets`:L249 · `CollectFuncContexts`:L280 · `CollectBodyErrors`:L288
`InferErrorConstructorType`:L315 · `VisitProgram`:L345 · `VisitSuite`:L355 · `VisitFunc_def`:L370
`VisitIsolate_stmt`:L402 · `VisitInline_foreign_block`:L416 · `VisitTry_stmt`:L435
`ContainsForeignConstruct`:L461 · `VisitLet_stmt`:L474 · `VisitVar_stmt`:L483 · `VisitConst_stmt`:L492
`HandleVarDecl`:L501 · `VisitGuard_stmt`:L510 · `TryExtractGuardCatch`:L562 · `VisitExpr_stmt`:L593
`VisitFlow_stmt`:L631 · `VisitCoalesce_expr`:L697 · `TryFindCatchMatch`:L757 · `FindFirstMatchStmt`:L770
`AnalyzeCatchMatch`:L781 · `ArmBodyProducesFailure`:L871 · `ExtractMatchedErrorNames`:L878 · `VisitAtom_expr`:L919
`VisitAtom`:L1048 · `VerifyQuestionPropagation`:L1074 · `IsErrorCompatibleOrAdaptable`:L1127
`ExpandErrorDomain`:L1159 · `EvaluateSimpleExpressionType`:L1182 · `UnwrapOkType`:L1196 · `ExtractErrorType`:L1199
`SplitUnionTypes`:L1204 · `AreTypesBroadlyCompatible`:L1207 · `IsNumeric`:L1221 · `IsIdentifier`:L1225
`Report`:L1228 · `AggregateResult`:L1236

### `Analysis/TypeSystem/PilotNullSafetyAnalyzer.cs` (1481 lines)
`ResolveEffectiveType`:L54 · `ResolveDeclaration`:L66 · `ClearNarrowing`:L79 · `CheckProgram`:L112
`IsOptionalType`:L118 · `IsDynamicOrAny`:L132 · `IsStrictNonNullableType`:L139 · `UnwrapFallibleType`:L153
`UnwrapOptionalType`:L160 · `EnsureOptionalType`:L174 · `PushScope`:L182 · `PopScope`:L183
`PrescanDeclarations`:L189 · `RegisterFunctionSignature`:L224 · `RegisterClassOrStruct`:L254
`RegisterStructDef`:L289 · `RegisterRecordDef`:L308 · `RegisterModelDef`:L339 · `RegisterExternDef`:L376
`HardenForeignReturnType`:L412 · `VisitErrorNode`:L431 · `VisitProgram`:L443 · `VisitSuite`:L453
`VisitFunc_def`:L466 · `VisitLet_stmt`:L501 · `VisitVar_stmt`:L511 · `VisitConst_stmt`:L521 · `HandleVarDecl`:L531
`VisitExpr_stmt`:L567 · `VisitFlow_stmt`:L644 · `VisitGuard_stmt`:L668 · `VisitIf_stmt`:L727
`VisitWhile_stmt`:L820 · `VisitMatch_stmt`:L866 · `VisitCoalesce_expr`:L892 · `VisitAtom_expr`:L926
`ResolveMemberType`:L1078 · `VisitTrailerArguments`:L1125 · `VisitAtom`:L1162 · `ApplyConditionNarrowing`:L1202
`TryExtractNullComparison`:L1222 · `FindAllComparisons`:L1232 · `CollectComparisons`:L1239
`TryExtractComparisonNarrowing`:L1250 · `BindPatternVariablesInScope`:L1311 · `BindNotPatternInScope`:L1324
`SuiteAlwaysExits`:L1383 · `SimpleStmtExits`:L1398 · `FindBranchPattern`:L1412 · `FindBranchGuard`:L1428
`ExtractElementType`:L1444 · `StripGenericSuffix`:L1455 · `IsSimpleIdentifierText`:L1462
`FormatDisplayType`:L1474 · `Report`:L1477

### `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (573 lines)
`ToString`:L30 · `RegisterBuiltins`:L55 · `PushScope`:L71 · `AggregateResult`:L78 · `VisitProgram`:L83
`PopScope`:L96 · `SetVariableType`:L104 · `TryLookupType`:L113 · `ParseTypeString`:L132 · `VisitLet_stmt`:L207
`VisitVar_stmt`:L235 · `VisitFunc_def`:L263 · `VisitTest`:L296 · `VisitCoalesce_expr`:L306 · `VisitArith_expr`:L339
`VisitAtom`:L364 · `VisitAtom_expr`:L415 · `VisitType_alias_stmt`:L458 · `VisitEnum_def`:L497
`VisitRecord_def`:L513 · `RegisterAdtVariantType`:L530 · `ExtractFieldTypes`:L552

### `Cli/Program.cs` (1229 lines)
`Main`:L20 · `PrintVersion`:L264 · `ExecuteInlineScript`:L275 · `ExecuteBuildSubcommand`:L288
`ExecuteCompilationSubcommand`:L404 · `RunScriptFile`:L490 · `ExecuteTestSubcommand`:L624
`ExecuteBenchmarkSubcommand`:L727 · `ExecutePatchSubcommand`:L787 · `ExecuteWatchSubcommand`:L869
`ExecutePackageSubcommand`:L914 · `ExecuteVenvSubcommand`:L1008 · `PrintUsage`:L1083 · `ParseAndReport`:L1120

### `CodeGen/Engine/PilotCodeGeneratorPipeline.cs` (410 lines)
`ShouldRun`:L39 · `Run`:L59 · `Run`:L73 · `ExtractScriptModel`:L165 · `ExtractClassModel`:L212
`ExtractMethodModel`:L287 · `ExtractDecorator`:L326 · `ExtractViaRegexFallback`:L352
`ExecuteInScriptGenerators`:L381

### `Compiler/PilotBytecodeCompiler.cs` (2464 lines)
`IsImmutable`:L41 · `AllocateTempSlot`:L48 · `RegisterBindingMetadata`:L51 · `HasDeclaredBinding`:L71
`EmitStoreBinding`:L82 · `EmitRestoreBinding`:L97 · `EmitLoadBindingOrUndefined`:L110 · `EmitLiteralConstant`:L123
`TryResolveAdtVariant`:L145 · `TryResolveAdtType`:L192 · `TryResolveRecordType`:L207
`IsKnownClassOrStructName`:L222 · `Compile`:L247 · `CompileSuite`:L259 · `PrescanProgramTypeDeclarations`:L303
`RegisterAdtCompileTime`:L343 · `RegisterRecordCompileTime`:L352 · `TryBuildTypeAliasAdtDescriptor`:L357
`BuildEnumDescriptor`:L445 · `BuildRecordDescriptor`:L473 · `ExtractGenericParamList`:L491
`ExtractSerializedFields`:L497 · `VisitProgram`:L534 · `VisitStmt`:L548 · `VisitSimple_stmt`:L557
`VisitSmall_stmt`:L572 · `VisitCompound_stmt`:L587 · `VisitType_alias_stmt`:L613 · `VisitEnum_def`:L634
`VisitRecord_def`:L652 · `VisitMatch_stmt`:L670 · `VisitGuard_stmt`:L676 · `VisitSuite`:L682 · `VisitLet_stmt`:L704
`VisitVar_stmt`:L726 · `VisitConst_stmt`:L748 · `ExtractBindingIdentifiers`:L770
`CompileVariableDeclaration`:L816 · `VisitExpr_stmt`:L842 · `AssignToTarget`:L886 · `CompileAugmentedAssign`:L952
`EmitAugassignOp`:L1080 · `VisitFlow_stmt`:L1098 · `VisitIf_stmt`:L1144 · `VisitWhile_stmt`:L1202
`VisitFor_stmt`:L1251 · `VisitFunc_def`:L1324 · `VisitTest`:L1373 · `VisitPipe_expr`:L1400
`TryCompileFluentPipeStage`:L1421 · `VisitSet_query_expr`:L1491 · `VisitCoalesce_expr`:L1500
`CompileCoalesceExprPrefix`:L1506 · `CompileCatchBlockAsExpression`:L1569 · `VisitOr_test`:L1587
`VisitAnd_test`:L1604 · `VisitNot_test`:L1621 · `VisitComparison`:L1633 · `VisitRange_expr`:L1671
`VisitBitor_expr`:L1688 · `VisitBitxor_expr`:L1702 · `VisitBitand_expr`:L1716 · `VisitShift_expr`:L1730
`VisitArith_expr`:L1745 · `VisitTerm`:L1760 · `VisitFactor`:L1782 · `VisitPower`:L1812 · `VisitAtom_expr`:L1825
`RequiresAtomExprAstFallback`:L1925 · `HasNamedArguments`:L1968 · `TryResolveKnownFieldOrderForCall`:L1979
`OrderCallArgumentExpressions`:L2019 · `VisitAtom`:L2067 · `VisitLiteral`:L2220 · `VisitCollection_expr`:L2351
`IsSimpleIdentifier`:L2384 · `GetAtomExpr`:L2400 · `CompileAtomExprPrefix`:L2408 · `UnescapeStringLiteral`:L2433
`StripQuotes`:L2449

### `Compiler/PilotCompiledProgram.cs` (607 lines)
`CheckVersionCompatibility`:L44 · `Run`:L85 · `Run`:L90 · `RunWithArgs`:L131 · `ExecuteCore`:L136 · `Save`:L222
`Save`:L253 · `ToBytes`:L264 · `Load`:L271 · `Load`:L362 · `FromBytes`:L368 · `ApplyHotPatch`:L385
`ApplyPatchSource`:L395 · `ApplyPatchFile`:L411 · `ApplyPatch`:L440 · `MergeProgramSources`:L457
`GetDeclarationName`:L574 · `ExtractStatementSource`:L587

### `Compiler/PilotMatchBytecodeLowerer.cs` (2247 lines)
`LowerMatch`:L71 · `AnalyzeArmBranches`:L162 · `ClassifyBranch`:L180 · `TryEmitAdtJumpTable`:L401
`TryEmitBoolJumpTable`:L491 · `TryEmitDenseIntJumpTable`:L551 · `EmitBranchChain`:L630 · `EmitArmBody`:L712
`SuiteLeavesExpressionOnStack`:L746 · `SimpleStmtLeavesExpressionOnStack`:L766 · `LowerIfStmt`:L781
`LowerWhileLetStmt`:L879 · `LowerGuardStmt`:L924 · `EmitCommitWithOptionalGuard`:L1071
`FindBranchPatternGuard`:L1118 · `LowerIsPatternExpr`:L1135 · `EmitPatternTest`:L1172 · `EmitAndPatternTest`:L1209
`EmitNotPatternTest`:L1225 · `EmitPrimaryPatternTest`:L1261 · `EmitRangePatternTest`:L1376
`EmitKeywordPatternTest`:L1414 · `EmitRelationalPatternTest`:L1449 · `EmitTypeOrVarPatternTest`:L1484
`EmitOrderedFieldSubpatterns`:L1741 · `EmitCollectionSlicePatternTest`:L1779 · `EmitPropertyPatternTest`:L1869
`StageSlotValue`:L1912 · `CommitStagedBindings`:L1936 · `PredeclarePatternBindingsInCompiler`:L1945
`CollectBoundVariables`:L1954 · `CollectFromPattern`:L1961 · `CollectFromNotPattern`:L1974
`IsIrrefutableSubpattern`:L2059 · `IsBareOptionalBindingPattern`:L2097 · `TryGetSimpleWildcard`:L2100
`IsSpreadSliceElement`:L2108 · `FindFieldIndex`:L2111 · `FindBranchPatternAndMutability`:L2126
`ExtractKeywordText`:L2158 · `IsPrimitiveTypeName`:L2166 · `TryParseLiteralInt`:L2174 · `TryExtractIntRange`:L2183
`EvaluateRangeBoundValue`:L2203 · `ParseSignedNumberValue`:L2221 · `ParseUnsignedIntToken`:L2232

### `Native/CAbi/PilotCAbiEmitter.cs` (515 lines)
`EmitHeader`:L24 · `EmitHeader`:L43 · `SanitizeCIdentifier`:L184 · `Extract`:L220 · `PrescanTypes`:L251
`ExtractGenerics`:L328 · `ExtractAdtFields`:L331 · `VisitFunc_def`:L354 · `VisitStruct_def`:L380
`VisitClass_def`:L397 · `MapType`:L438

### `Native/Llvm/PilotLlvmEmitter.cs` (1869 lines)
`Emit`:L50 · `Emit`:L69 · `NextReg`:L75 · `NextBlock`:L76 · `CompileProgram`:L78 · `PrescanTypesAndFunctions`:L187
`ExtractGenericNames`:L321 · `ExtractAdtFields`:L324 · `CompileFunction`:L347 · `DefaultZeroLiteral`:L408
`VisitStmt`:L418 · `VisitCompound_stmt`:L425 · `VisitSimple_stmt`:L436 · `VisitSmall_stmt`:L450
`VisitLet_stmt`:L460 · `VisitVar_stmt`:L467 · `VisitConst_stmt`:L474 · `CompileVariableDecl`:L481
`VisitExpr_stmt`:L526 · `VisitFlow_stmt`:L599 · `VisitIf_stmt`:L642 · `VisitWhile_stmt`:L683 · `VisitFor_stmt`:L718
`VisitMatch_stmt`:L789 · `CompileMatchInternal`:L795 · `EmitLlvmMatchArmBodies`:L1015
`InferMatchResultType`:L1070 · `TryExtractArmResultExpr`:L1085 · `IsWildcardOrCatchAllArm`:L1098
`TryMatchArmToVariant`:L1119 · `ExtractSimpleBindingName`:L1160 · `ExtractSwitchIntegersFromPattern`:L1170
`VisitSuite`:L1212 · `CompileExpression`:L1236 · `CompileExpressionNode`:L1246 · `CompilePipeExpr`:L1254
`CompileComparison`:L1264 · `CompileRangeExpr`:L1316 · `CompileBitorExpr`:L1322 · `CompileBitandExpr`:L1343
`CompileArithExpr`:L1364 · `CompileTerm`:L1404 · `CompileFactor`:L1451 · `CompilePower`:L1483
`CompileAtomExpr`:L1488 · `EmitAdtVariantConstruction`:L1638 · `EmitRecordConstruction`:L1696
`CompileLiteral`:L1729 · `GetOrAddGlobalString`:L1784 · `EnsureI1`:L1815 · `MapType`:L1823 · `StripQuotes`:L1858

### `Native/PilotNativeCompiler.cs` (373 lines)
`CreateSuccess`:L55 · `CreateFailure`:L71 · `Compile`:L95 · `CompileFile`:L194 · `BuildNativeExecutable`:L216
`CompileViaCToolchain`:L287 · `ProbeCompilerToolchain`:L331 · `IsCommandAvailable`:L347

### `Native/Wasm/PilotWasmEmitter.cs` (878 lines)
`EmitWat`:L31 · `EmitWasm`:L51 · `CompileProgram`:L57 · `CompileFunction`:L148 · `VisitStmt`:L196
`VisitSimple_stmt`:L203 · `VisitSmall_stmt`:L217 · `VisitLet_stmt`:L227 · `VisitVar_stmt`:L234
`VisitConst_stmt`:L241 · `CompileVarDecl`:L248 · `VisitExpr_stmt`:L265 · `VisitFlow_stmt`:L324
`VisitIf_stmt`:L341 · `VisitWhile_stmt`:L358 · `VisitFor_stmt`:L374 · `VisitSuite`:L421 · `CompileExpression`:L445
`CompilePipeExpr`:L455 · `CompileComparison`:L465 · `CompileBitorExpr`:L494 · `CompileBitandExpr`:L511
`CompileArithExpr`:L528 · `CompileTerm`:L547 · `CompileFactor`:L573 · `CompileAtomExpr`:L592
`CompileLiteral`:L650 · `MapType`:L692 · `AssembleWatToWasm`:L714 · `WriteSection`:L832 · `WriteString`:L839
`WriteVarUInt`:L846 · `WriteVarInt`:L857

### `Pilot.LanguageServer/Lsp/PilotLspServer.cs` (747 lines)
`RunAsync`:L100 · `ProcessMessage`:L121 · `UpdateDocumentAndBuildDiagnostics`:L498 · `TryGetAnalysis`:L531
`ConvertToDocumentSymbol`:L548 · `GetCompletionItemKind`:L567 · `CreateRange`:L582 · `BuildResponse`:L588
`ExtractWordAt`:L599 · `SendRawJson`:L646 · `ReadMessageAsync`:L664

### `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (4641 lines)
`IsBuiltinType`:L48 · `Analyze`:L274 · `CheckUnusedVariables`:L348 · `PreRegisterTopLevelDeclarations`:L385
`RegisterFunctionStub`:L475 · `RegisterTypeStub`:L500 · `DeclareBindingPattern`:L524 · `VisitLet_stmt`:L583
`VisitVar_stmt`:L598 · `VisitConst_stmt`:L633 · `VisitRef_stmt`:L648 · `VisitDynamic_stmt`:L663
`VisitAnnassign_stmt`:L671 · `VisitType_alias_stmt`:L702 · `VisitPlainTypeAlias`:L711 · `VisitExpr_stmt`:L724
`CheckImmutableMemberAssignment`:L834 · `StripGenericSuffixForLookup`:L889 · `FindFirstAtomExpr`:L896
`VisitGuard_stmt`:L907 · `VisitDefer_stmt`:L974 · `VisitFunc_def`:L980 · `VisitContract_clause`:L1125
`VisitPattern_assign_stmt`:L1138 · `VisitClass_def`:L1147 · `VisitStruct_def`:L1179 · `VisitInterface_def`:L1208
`VisitEnum_def`:L1224 · `VisitActor_def`:L1266 · `VisitModel_def`:L1282 · `VisitProperty_def`:L1299
`VisitOperator_def`:L1374 · `VisitExtension_def`:L1411 · `VisitAdapter_def`:L1430 · `VisitUnit_stmt`:L1460
`VisitDsl_stmt`:L1476 · `VisitMacro_def`:L1492 · `VisitQuote_stmt`:L1523 · `VisitQuote_expr`:L1533
`VisitQuasiquote_expr`:L1542 · `VisitSplice_expr`:L1554 · `VisitForeign_import_stmt`:L1566
`VisitExtern_def`:L1661 · `ProcessExternMember`:L1724 · `ProcessExternFunction`:L1740 · `ProcessExternType`:L1816
`VisitInline_foreign_block`:L1870 · `VisitDatabase_stmt`:L1888 · `VisitTransaction_stmt`:L1899
`VisitSavepoint_stmt`:L1912 · `VisitSavepoint_block`:L1923 · `VisitRollback_stmt`:L1935 · `VisitNoalloc_stmt`:L1957
`VisitWhile_stmt`:L1967 · `VisitFor_stmt`:L2001 · `VisitIf_stmt`:L2080 · `VisitFlow_stmt`:L2100
`VisitPass_stmt`:L2216 · `VisitAssert_stmt`:L2223 · `VisitComparison`:L2230 · `VisitComp_op`:L2245
`VisitSelect_stmt`:L2253 · `VisitSelect_arm`:L2272 · `VisitQuery_expr`:L2296 · `VisitSql_query_expr`:L2345
`IsInsideSyncFunction`:L2373 · `VisitUnary_op`:L2390 · `VisitPrimary_type`:L2422 · `VisitAtom_expr`:L2457
`VisitTerm`:L2570 · `VisitLabeled_stmt`:L2596 · `VisitBlock`:L2606 · `VisitLambda_expr`:L2615
`VisitMacro_call`:L2699 · `VisitSymbol_literal`:L2711 · `VisitCascade_trailer`:L2724 · `VisitNameof_expr`:L2763
`VisitAtom`:L2782 · `InferLiteralType`:L2970 · `ValidateAndResolveType`:L2993 · `IsCompatibleType`:L3021
`IsStrictPrimitive`:L3117 · `NormalizePrimitive`:L3123 · `TryExtractUnit`:L3131 · `ExpandUnitExpression`:L3144
`SuiteGuaranteesExit`:L3155 · `SimpleStmtExits`:L3168 · `VisitSimple_stmt`:L3173 · `VisitSuite`:L3207
`StmtGuaranteesExit`:L3248 · `VisitArith_expr`:L3264 · `VisitExcept_clause`:L3304 · `VisitMatch_stmt`:L3325
`VisitMatch_arm`:L3348 · `VisitMatchArmWithScrutinee`:L3353 · `BindPatternIdentifiers`:L3395
`VisitRecord_def`:L3401 · `RegisterAdtSumTypeAlias`:L3459 · `RegisterAdtVariantSymbol`:L3500
`FinishGuardSuite`:L3538 · `VisitIfBranchesWithPatterns`:L3556 · `BindPatternVarSingle`:L3665
`BindPatternNode`:L3688 · `BindPatternNodeLegacyUnused`:L3856 · `RegisterGenericParams`:L3891
`DeclareVariableSymbol`:L3903 · `DeclareVariableSymbolFromToken`:L3918 · `CreateSymbol`:L3952
`CreateSymbolFromToken`:L3974 · `RegisterSymbol`:L4024 · `AddSymbolReferenceDirect`:L4051
`AddSemanticTokenDirect`:L4063 · `ProcessFStringNode`:L4084 · `AddSymbolReference`:L4222 · `AddSemanticToken`:L4235
`AddDiagnostic`:L4248 · `AddDiagnosticDirect`:L4271 · `GetSemanticTokenTypeForSymbol`:L4293
`GetTokenCoords`:L4310 · `ExtractDictTypes`:L4319 · `ExtractElementType`:L4347 · `IsSubtypeOf`:L4378
`FindCommonBaseType`:L4391 · `HomogenizeTypes`:L4425 · `TryExtractSimpleIdentifier`:L4498
`FindFirstIdentifier`:L4507 · `Truncate`:L4518 · `EnterScope`:L4521 · `Dispose`:L4567 · `SyntaxError`:L4575

### `Pilot.LanguageServer/Semantics/PilotSymbol.cs` (392 lines)
`GetLspDocumentSymbolKind`:L189 · `FormatHoverMarkdown`:L212 · `Resolve`:L286 · `GetAllVisibleSymbols`:L296
`FindSymbolAtPosition`:L320 · `FindScopeAtLine`:L331 · `FindDeepestScope`:L336 · `EncodeSemanticTokens`:L346

### `Preprocessing/PilotPreprocessor.cs` (983 lines)
`ToString`:L24 · `IsWarningSuppressed`:L69 · `Preprocess`:L103 · `ProcessDirective`:L219 · `ProcessPragma`:L447
`EvaluateCondition`:L490 · `SplitLines`:L518 · `IsValidSymbolName`:L556 · `StripQuotes`:L571
`TokenizeCondition`:L588 · `Parse`:L661 · `ParseOr`:L669 · `ParseAnd`:L681 · `ParseEquality`:L693
`ParseUnary`:L705 · `ParsePrimary`:L715 · `GetIndent`:L767 · `FindTrailingCommentIndex`:L779
`IsBlockStartLine`:L801 · `EndsWithBlockColon`:L831 · `DesugarColonBlocks`:L877

### `Runtime/Builtins/PilotBuiltinRegistry.cs` (935 lines)
`Register`:L25 · `Register`:L30 · `TryGet`:L38 · `GetAll`:L41 · `RegisterDefaults`:L43 · `GetOperatorFunction`:L913

### `Runtime/Bytecode/PilotBytecodeChunk.cs` (586 lines)
`Emit`:L89 · `EmitJump`:L97 · `PatchJump`:L107 · `AddJumpTable`:L121 · `AddTypeDescriptor`:L130
`AddConstant`:L139 · `AddIdentifier`:L155 · `AddNestedChunk`:L171 · `AddAstFallback`:L180
`GetOrAllocateLocalSlot`:L189 · `TryGetLocalSlot`:L203 · `Write`:L210 · `Read`:L304 · `ReadV2Extensions`:L392
`WriteValue`:L453 · `ReadValue`:L522 · `ReadKeywordVariant`:L545 · `ReadList`:L554 · `ReadTuple`:L565
`ReadRange`:L576

### `Runtime/Bytecode/PilVirtualMachine.cs` (1483 lines)
`Execute`:L28 · `DeclareTypeDescriptorInContext`:L1128 · `RefreshLayouts`:L1181 · `CheckRuntimeTypeMatch`:L1193
`MatchesClassHierarchy`:L1289 · `EnsureStackCapacity`:L1310 · `InvokeCallable`:L1319 · `InvokeMethod`:L1358
`CreateIterator`:L1375 · `EnumerateRange`:L1416 · `EnumerateString`:L1430 · `EnumerateObjects`:L1438
`EnumerateSingle`:L1446 · `CreateParameters`:L1451

### `Runtime/Execution/PilotEngine.cs` (650 lines)
`Execute`:L21 · `Execute`:L27 · `ExecuteFile`:L153 · `EvaluateExpression`:L168 · `InvokeFunction`:L186
`CreateGenerator`:L341 · `CreateGenerator`:L381 · `RunnerBody`:L386 · `Next`:L441 · `Close`:L471
`TranspileTo`:L486 · `TranspileFrom`:L492 · `Compile`:L498 · `CompileFile`:L508 · `CreateCallScope`:L518
`SnapshotOldCalls`:L581 · `Write`:L627 · `Write`:L633 · `WriteLine`:L639 · `Flush`:L645

### `Runtime/Execution/PilotInterpreterVisitor.cs` (5145 lines)
`VisitProgram`:L84 · `VisitStmt`:L129 · `VisitSimple_stmt`:L143 · `VisitSuite`:L160 · `VisitLet_stmt`:L184
`VisitVar_stmt`:L192 · `VisitConst_stmt`:L200 · `DeclareBinding`:L208 · `DeclareRecordPattern`:L236
`ProcessRecordRestPattern`:L264 · `ProcessRecordFieldPattern`:L296 · `DeclareListPattern`:L358
`DeclareTuplePattern`:L431 · `VisitPattern_assign_stmt`:L504 · `VisitExpr_stmt`:L522 · `TryUnwrapAtomExpr`:L576
`AnalyzeTarget`:L616 · `EvaluateAtomExprPrefix`:L693 · `AssignTarget`:L778 · `VisitFunc_def`:L913
`ContainsYield`:L1011 · `ContainsYieldCore`:L1017 · `VisitClass_def`:L1034 · `VisitState_def`:L1199
`VisitMacro_def`:L1252 · `VisitType_alias_stmt`:L1280 · `VisitEnum_def`:L1343 · `VisitRecord_def`:L1362
`VisitStruct_def`:L1401 · `DefaultValueForType`:L1461 · `RegisterAdtDefinition`:L1470
`RefreshAllAdtAndRecordLayouts`:L1493 · `IsKnownTypeOrBinding`:L1505 · `ExtractGenericParamNames`:L1521
`ExtractAdtFields`:L1527 · `VisitExtension_def`:L1556 · `EnsureSelfParameter`:L1669
`SanitizeExtensionTargetName`:L1682 · `VisitAdapter_def`:L1692 · `VisitSteal_stmt`:L1739
`VisitDelegate_stmt`:L1779 · `VisitContract_forward_stmt`:L1800 · `VisitIf_stmt`:L1825
`HasBranchSomeKeyword`:L1905 · `FindBranchPattern`:L1925 · `FindBranchPatternGuard`:L1944
`IsBareOptionalBindingPattern`:L1963 · `VisitWhile_stmt`:L1988 · `VisitLabeled_stmt`:L2114 · `VisitBlock`:L2145
`VisitFor_stmt`:L2168 · `TryGetSimpleForTargetName`:L2522 · `VisitWith_stmt`:L2540 · `AssignForTarget`:L2621
`VisitMatch_stmt`:L2647 · `EvaluateMatchSuite`:L2676 · `VisitGuard_stmt`:L2703 · `EvaluateGuardRhs`:L2802
`VisitDefer_stmt`:L2849 · `IsKnownExceptionTypeName`:L2906 · `VisitTry_stmt`:L2916 · `IsExceptionMatching`:L3033
`VisitAssert_stmt`:L3057 · `VisitFlow_stmt`:L3071 · `VisitTestlist`:L3200 · `VisitTest`:L3217
`VisitLambda_expr`:L3242 · `VisitPipe_expr`:L3318 · `VisitSet_query_expr`:L3335 · `VisitCoalesce_expr`:L3344
`VisitCoalesceExprPrefix`:L3349 · `EvaluateCatchClause`:L3386 · `VisitOr_test`:L3423 · `VisitAnd_test`:L3437
`VisitNot_test`:L3451 · `VisitComparison`:L3460 · `VisitRange_expr`:L3503 · `VisitBitor_expr`:L3518
`VisitBitxor_expr`:L3532 · `VisitBitand_expr`:L3546 · `VisitShift_expr`:L3560 · `VisitArith_expr`:L3577
`VisitTerm`:L3594 · `EvaluateProjection`:L3646 · `VisitFactor`:L3706 · `VisitUnary_op`:L3715 · `VisitPower`:L3754
`VisitAtom_expr`:L3769 · `CreateFluentMemberFunction`:L3906 · `TryInvokeDirectMemberCall`:L3948
`EvaluateCascade`:L4079 · `DispatchCall`:L4128 · `InvokeCallableSingle`:L4247 · `ExtractCollectionItems`:L4352
`EvaluateCallArguments`:L4407 · `EvaluateArglist`:L4413 · `BindArgumentsToFunction`:L4418
`IsCollectionOrTupleParameterType`:L4498 · `VisitAtom`:L4528 · `VisitSymbol_literal`:L4555
`ResolveIdentifier`:L4568 · `VisitLiteral`:L4588 · `ParseDurationLiteralSeconds`:L4697
`ParseFrequencyLiteralHz`:L4715 · `ParseMemoryLiteralBytes`:L4729 · `IsSimpleIdentifier`:L4745
`ParseFStringSegments`:L4758 · `EvaluateFString`:L4804 · `UnescapeString`:L4848 · `VisitCollection_expr`:L4861
`VisitDictorsetmaker`:L4892 · `VisitTestlist_comp`:L4975 · `VisitInline_foreign_block`:L4991
`GetBlockBodyText`:L5112

### `Runtime/Execution/PilotPatternMatcher.cs` (970 lines)
`Match`:L25 · `MatchPatternInternal`:L46 · `MatchOrPattern`:L57 · `MatchAndPattern`:L86 · `MatchNotPattern`:L103
`MatchPrimaryPattern`:L134 · `MatchRangePattern`:L249 · `EvaluateRangeBound`:L295 · `ParseSignedNumber`:L315
`ParseUnsignedIntToken`:L326 · `MatchKeywordPattern`:L335 · `MatchTypeOrVarPattern`:L396 · `MatchAdtFields`:L716
`MatchRecordFields`:L764 · `MatchPropertyPattern`:L803 · `MatchesObjectType`:L847 · `MatchesPrimitiveType`:L875
`MatchCollectionSlice`:L898

### `Runtime/HotReload/PilotHotReloader.cs` (732 lines)
`CreateNonExistent`:L50 · `Capture`:L54 · `Restore`:L107 · `ToPilotMap`:L190 · `ToPilotMap`:L218
`ApplyCodeOrFile`:L262 · `ApplySourcePatch`:L287 · `ApplyCompiledPatch`:L311 · `ApplyPatchFile`:L459
`Rollback`:L522 · `WatchFile`:L573 · `UnwatchAll`:L634 · `Dispose`:L645 · `IsCompiledBinary`:L650
`RestoreScopeChain`:L663 · `ExtractTopLevelDeclaredSymbols`:L689 · `ComputeSha256Hex`:L727

### `Runtime/Interop/PolyglotRegistry.cs` (670 lines)
`Register`:L34 · `TryGet`:L40 · `NormalizeLanguage`:L46 · `Get`:L58 · `ExecuteBlock`:L82 · `Invoke`:L108
`Import`:L126 · `EvaluatePythonLiteralOrMarshal`:L131 · `ExecuteBlock`:L159 · `Invoke`:L183 · `Import`:L198
`EvaluateCSharpLiteralOrMarshal`:L203 · `ExecuteBlock`:L231 · `Invoke`:L250 · `Import`:L255 · `ExecuteBlock`:L274
`Invoke`:L308 · `Import`:L311 · `ExecuteBlock`:L328 · `Invoke`:L360 · `Import`:L363 · `ExecuteBlock`:L380
`Invoke`:L429 · `Import`:L432 · `ExecuteBlock`:L449 · `Invoke`:L474 · `Import`:L477 · `ExecuteBlock`:L494
`Invoke`:L518 · `Import`:L521 · `ExecuteBlock`:L538 · `Invoke`:L568 · `Import`:L571 · `ExecuteBlock`:L588
`ExecuteSqlQuery`:L593 · `Invoke`:L642 · `Import`:L645 · `ExecuteBlock`:L662 · `Invoke`:L665 · `Import`:L668

### `Runtime/Interop/PolyglotTypeBridge.cs` (442 lines)
`RegisterCustomMapper`:L24 · `ToPilotValue`:L30 · `TryMapPrimitiveToPilotValue`:L66
`TryMapCollectionToPilotValue`:L145 · `ToForeignValue`:L201 · `MapKindToForeignValue`:L226
`TryCoerceToForeignType`:L250 · `ConvertPocoToPilotObject`:L280 · `ConvertListToHost`:L326
`ConvertMapToHost`:L336 · `InferForeignReturnPilotType`:L352 · `InferForeignClrPilotType`:L383
`InferSqlColumnPilotType`:L407 · `MapClrBaseType`:L430

### `Runtime/Packaging/PilotPackageManager.cs` (376 lines)
`RegisterAdapter`:L34 · `TryGetAdapter`:L42 · `Init`:L50 · `Add`:L85 · `Remove`:L123 · `Restore`:L159 · `List`:L246
`Pack`:L296 · `ParsePackageSpec`:L338 · `ComputeHash`:L361 · `GetEffectiveModulesDirectory`:L367

### `Runtime/Values/PilotAdt.cs` (1037 lines)
`PilotFieldMemoryLayout`:L35 · `PilotVariantMemoryLayout`:L46 · `ComputeForAdt`:L90 · `ComputeForRecord`:L155
`FormsDirectValueCycle`:L201 · `ComputeOptionLayout`:L282 · `ComputeResultLayout`:L308
`TrySplitFallibleTypeSpec`:L334 · `HasZeroAllocationNiche`:L357 · `EstimateInlineTypeSizeAndAlign`:L421
`StripGenericsAndModifiers`:L512 · `SplitTopLevel`:L527 · `AlignUp`:L551 · `GetFieldIndex`:L625
`Instantiate`:L634 · `CoerceFieldValue`:L706 · `ToString`:L715 · `AddVariant`:L738 · `TryGetVariant`:L747
`TryGetVariantByTag`:L750 · `ToString`:L761 · `Equals`:L779 · `Equals`:L796 · `GetHashCode`:L798 · `ToString`:L810
`IsFloatTypeName`:L873 · `GetFieldIndex`:L876 · `Instantiate`:L879 · `ToString`:L945 · `Equals`:L962
`Equals`:L974 · `GetHashCode`:L976 · `ToString`:L987 · `Equals`:L1013 · `Equals`:L1025 · `GetHashCode`:L1027
`ToString`:L1035

### `Runtime/Values/PilotMethodGroup.cs` (551 lines)
`AddOverload`:L53 · `CloneSnapshot`:L71 · `RestoreFromSnapshot`:L77 · `Union`:L90 · `BindTo`:L105 · `Invoke`:L114
`ExtractCollectionElements`:L246 · `ExecuteOverload`:L254 · `TryMatchOverload`:L272 · `CanPartiallyMatch`:L358
`IsTypeCompatible`:L395 · `IsTypeCompatible`:L400 · `IsDirectlyCompatible`:L421 · `GetOverloadValues`:L500
`BuildSignature`:L531 · `ToString`:L549

### `Runtime/Values/PilotObject.cs` (649 lines)
`Union`:L31 · `Subtract`:L59 · `Subtract`:L93 · `Intersect`:L107 · `CloneWithName`:L134 · `MergeMethodValue`:L151
`CloneSnapshot`:L209 · `RestoreFromSnapshot`:L227 · `StealFrom`:L295 · `DelegateContract`:L330 · `Instantiate`:L345
`PopulateDefaultFields`:L373 · `ToString`:L387 · `GetMember`:L413 · `SetMember`:L557 · `BindMethodValue`:L562
`BindMethod`:L595 · `Clone`:L597 · `Equals`:L607 · `Equals`:L621 · `GetHashCode`:L623 · `ToString`:L635

### `Runtime/Values/PilotValue.cs` (2103 lines)
`CreateTable`:L38 · `FromBool`:L63 · `FromInt`:L64 · `FromUInt`:L65 · `FromFloat`:L66 · `FromDecimal`:L67
`FromChar`:L68 · `FromString`:L69 · `FromKeyword`:L70 · `FromKeyword`:L71 · `FromList`:L72 · `FromTuple`:L73
`FromMap`:L74 · `FromSet`:L75 · `FromRange`:L76 · `FromMatrix`:L77 · `FromFunction`:L78 · `FromObject`:L79
`FromGenerator`:L80 · `FromLazy`:L81 · `FromFuture`:L82 · `FromForeign`:L83 · `FromMethodGroup`:L84
`FromNativeArray`:L85 · `FromNativeMap`:L86 · `FromAdtType`:L87 · `FromAdtUnit`:L88 · `FromAdtBoxed`:L89
`FromRecordUnboxedEmpty`:L90 · `FromRecordUnboxedPairF64`:L91 · `FromRecordBoxed`:L93 · `FromKeywordVariant`:L94
`FromSome`:L102 · `UnwrapOption`:L119 · `FromFailure`:L139 · `UnwrapFailure`:L159
`TryCreateUnboxedAdtSingle`:L191 · `PilotValue`:L214 · `PilotValue`:L215 · `PilotValue`:L216 · `PilotValue`:L217
`PilotValue`:L218 · `PilotValue`:L219 · `PilotValue`:L220 · `PilotValue`:L221 · `PilotValue`:L222 · `AsInt`:L250
`AsFloat`:L262 · `AsString`:L274 · `AsDecimal`:L275 · `AsChar`:L276 · `AsKeyword`:L277 · `AsList`:L279
`AsTuple`:L280 · `AsMap`:L281 · `AsSet`:L282 · `AsRange`:L283 · `AsMatrix`:L284 · `AsFunction`:L285
`AsObject`:L286 · `AsGenerator`:L287 · `AsLazy`:L288 · `AsFuture`:L289 · `AsMethodGroup`:L290
`AsNativeArray`:L291 · `AsNativeMap`:L292 · `AsForeign`:L293 · `AsAdtType`:L301 · `AsKeywordVariant`:L302
`GetAdtVariantDefinition`:L304 · `GetAdtTag`:L312 · `GetAdtFieldCount`:L314 · `GetAdtField`:L316
`GetRecordDefinition`:L339 · `GetRecordFieldCount`:L346 · `GetRecordField`:L348 · `CreateClassConstructor`:L368
`GetMember`:L687 · `CreateParseError`:L1189 · `ComputeCollectionSum`:L1198 · `InvokeSingleCallback`:L1221
`MapCollection`:L1238 · `FilterCollection`:L1248 · `TryInvokePrimitiveIntrinsic`:L1266
`EnumerateCollection`:L1739 · `CreateAppendFunction`:L1747 · `ResolveExtensionOrRoot`:L1793
`ResolveRootMember`:L1831 · `CloneValue`:L1884 · `Equals`:L1900 · `EqualsAdt`:L1945 · `EqualsRecord`:L1960
`ComputeAdtHashCode`:L1975 · `ComputeRecordHashCode`:L1985 · `FormatAdt`:L1994 · `FormatRecord`:L2004
`EqualsList`:L2018 · `EqualsTuple`:L2025 · `EqualsMap`:L2032 · `Equals`:L2042 · `GetHashCode`:L2044
`ToString`:L2059 · `FormatNestedSome`:L2095

### `Runtime/Venv/PilotVirtualEnvironment.cs` (446 lines)
`CheckVersionCompatibility`:L41 · `Load`:L44 · `Save`:L91 · `Create`:L157 · `Detect`:L220 · `Activate`:L252
`Deactivate`:L291 · `GenerateActivationScripts`:L299 · `GeneratePowerShellScript`:L306 · `GenerateCmdScripts`:L348
`GeneratePosixScript`:L381 · `GenerateLauncher`:L422

### `Transpilation/PilotLibraryMapper.cs` (524 lines)
`UsesMatrixPrimitives`:L35 · `UsesJsonPrimitives`:L43 · `TranslateForeignLibrariesToPilot`:L57
`MapNumpyDtypeToPilot`:L178 · `TranslatePilotLibrariesToTarget`:L203 · `MapPilotTypeToNumpy`:L270
`GetMatrixRuntimePreamble`:L291

### `Transpilation/PilotOptimizer.cs` (398 lines)
`AnalyzeMutatedVariables`:L36 · `CollectMutations`:L43 · `IsSimpleIdentifier`:L78 · `FoldConstantsAndNormalize`:L96
`NormalizeDimensionalLiterals`:L131 · `TryFoldSimpleNumericExpression`:L194 · `TryParseRangeExpr`:L222
`TryParseRangeCall`:L240 · `OptimizeIngestedPilotCode`:L269 · `InvertConditionForGuard`:L354

### `Transpilation/Targets/CppTargetConverter.cs` (581 lines)
`TranspileSource`:L28 · `Transpile`:L56 · `IngestToPilot`:L78 · `ConvertCppLineToPilot`:L113
`MapCppTypeToPilot`:L215 · `VisitProgram`:L246 · `VisitStmt`:L282 · `VisitSimpleStmt`:L289 · `VisitSmallStmt`:L303
`VisitCompoundStmt`:L357 · `VisitGuardStmt`:L369 · `VisitFuncDef`:L380 · `VisitClassDef`:L406 · `VisitIfStmt`:L420
`VisitWhileStmt`:L447 · `VisitForStmt`:L456 · `VisitForeignBlock`:L480 · `VisitSuiteBody`:L495
`TranslateExprStmt`:L518 · `TranslateExpr`:L534 · `MapType`:L543

### `Transpilation/Targets/CSharpTargetConverter.cs` (422 lines)
`TranspileSource`:L20 · `Transpile`:L48 · `IngestToPilot`:L76 · `ConvertCSharpLineToPilot`:L110
`VisitProgram`:L165 · `VisitStmt`:L179 · `VisitSimpleStmt`:L186 · `VisitSmallStmt`:L200 · `VisitCompoundStmt`:L253
`VisitFuncDef`:L264 · `VisitClassDef`:L289 · `VisitIfStmt`:L301 · `VisitWhileStmt`:L325 · `VisitForStmt`:L333
`VisitMatchStmt`:L343 · `VisitSuite`:L365 · `MapType`:L392

### `Transpilation/Targets/CTargetConverter.cs` (577 lines)
`TranspileSource`:L23 · `Transpile`:L51 · `IngestToPilot`:L72 · `ConvertCLineToPilot`:L105 · `MapCTypeToPilot`:L188
`VisitProgram`:L219 · `VisitStmt`:L255 · `VisitSimpleStmt`:L262 · `VisitSmallStmt`:L276 · `VisitCompoundStmt`:L330
`VisitGuardStmt`:L342 · `VisitFuncDef`:L353 · `VisitClassDef`:L380 · `VisitIfStmt`:L390 · `VisitWhileStmt`:L417
`VisitForStmt`:L426 · `VisitForeignBlock`:L450 · `VisitSuiteBody`:L465 · `TranslateExprStmt`:L488
`TranslateExpr`:L515 · `InferCType`:L524 · `MapType`:L537

### `Transpilation/Targets/JavaScriptTargetConverter.cs` (495 lines)
`TranspileSource`:L27 · `Transpile`:L55 · `IngestToPilot`:L73 · `ConvertJsLineToPilot`:L106 · `VisitProgram`:L175
`VisitStmt`:L189 · `VisitSimpleStmt`:L196 · `VisitSmallStmt`:L210 · `VisitCompoundStmt`:L266
`VisitGuardStmt`:L279 · `VisitFuncDef`:L290 · `VisitClassDef`:L313 · `VisitIfStmt`:L326 · `VisitWhileStmt`:L353
`VisitForStmt`:L362 · `VisitMatchStmt`:L387 · `VisitForeignBlock`:L415 · `VisitSuiteBody`:L430
`TranslateExpr`:L453

### `Transpilation/Targets/LuaTargetConverter.cs` (528 lines)
`TranspileSource`:L22 · `Transpile`:L50 · `IngestToPilot`:L67 · `ConvertLuaLineToPilot`:L109 · `VisitProgram`:L193
`VisitStmt`:L207 · `VisitSimpleStmt`:L214 · `VisitSmallStmt`:L228 · `VisitCompoundStmt`:L288
`VisitGuardStmt`:L301 · `VisitFuncDef`:L312 · `VisitClassDef`:L333 · `VisitIfStmt`:L345 · `VisitWhileStmt`:L370
`VisitForStmt`:L379 · `VisitMatchStmt`:L404 · `VisitForeignBlock`:L436 · `VisitSuite`:L451 · `TranslateExpr`:L474

### `Transpilation/Targets/PythonTargetConverter.cs` (531 lines)
`TranspileSource`:L26 · `Transpile`:L54 · `IngestToPilot`:L73 · `ConvertPythonLineToPilot`:L138
`VisitProgram`:L184 · `VisitStmt`:L198 · `VisitSimpleStmt`:L205 · `VisitSmallStmt`:L219 · `VisitCompoundStmt`:L273
`VisitGuardStmt`:L286 · `VisitFuncDef`:L296 · `VisitClassDef`:L333 · `VisitIfStmt`:L345 · `VisitWhileStmt`:L369
`VisitForStmt`:L377 · `VisitMatchStmt`:L401 · `VisitForeignBlock`:L420 · `VisitSuite`:L435 · `MapType`:L463
`TranslateExpr`:L485 · `TranslatePattern`:L524

### `Transpilation/Targets/RustTargetConverter.cs` (479 lines)
`TranspileSource`:L28 · `Transpile`:L56 · `IngestToPilot`:L74 · `ConvertRustLineToPilot`:L102 · `VisitProgram`:L151
`VisitStmt`:L165 · `VisitSimpleStmt`:L172 · `VisitSmallStmt`:L186 · `VisitCompoundStmt`:L259
`VisitGuardStmt`:L272 · `VisitFuncDef`:L282 · `VisitClassDef`:L307 · `VisitIfStmt`:L317 · `VisitWhileStmt`:L341
`VisitForStmt`:L349 · `VisitMatchStmt`:L364 · `VisitForeignBlock`:L387 · `VisitSuite`:L402 · `FormatExpr`:L429
`MapType`:L447

### `vscode-pilot/src/extension.ts` (700 lines)
`pickNewestExistingFile`:L18 · `resolvePilotExecutionPrefix`:L40 · `activate`:L321 · `addOrUpdateLaunchJson`:L622
`deactivate`:L700

---

## 4. Complete Codebase File & Type Symbol Inventory

### .agents
- `.agents/hooks.json` (23 L)
- `.agents/scripts/sync_ai_index_hook.ps1` (41 L)
- `.agents/scripts/sync_version.ps1` (46 L)
- `.agents/scripts/update_ai_index.ps1` (311 L)

### .jules
- `.jules/bolt.md` (8 L)

### (Repository Root)
- `.gitignore` (74 L)
- `AGENTS.md` (51 L)
- `fix_type.sh` (4 L)
- `input.txt` (187 L)
- `MILESTONES.md` (717 L)
- `Pilot.csproj` (147 L)
- `Pilot.slnx` (5 L)
- `README.md` (756 L)

### Analysis
- `Analysis/ContractVerification/PilotContractVerifier.cs` (227 L) — class `PilotContractVerifier`:L14, class `FunctionContractInfo`:L21
- `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs` (344 L) — class `UnitDimensionSignature`:L13, class `PilotDimensionChecker`:L101
- `Analysis/Loops/PilotForLoopAnalyzer.cs` (473 L) — enum `LoopScheduleMode`:L11, enum `LoopReturnMode`:L26, class `PilotForLoopInfo`:L41, class `PilotForLoopAnalyzer`:L58, class `ScopedBodyScanState`:L453
- `Analysis/Ownership/PilotBorrowChecker.cs` (503 L) — enum `ResourceState`:L13, enum `ResourceAffinity`:L25, class `ResourceBinding`:L44, class `PilotBorrowChecker`:L68
- `Analysis/PatternMatching/PilotExhaustivenessChecker.cs` (2046 L) — class `PilotExhaustivenessChecker`:L21, record `MatchPat`:L536, record `WildPat`:L537, record `OrPat`:L541, record `CtorPat`:L542, enum `CtorFamily`:L544, record `CtorDesc`:L563
- `Analysis/PilotAnalysisPipeline.cs` (148 L) — class `PilotStaticAnalysisResult`:L17, class `PilotAnalysisPipeline`:L46, class `NullAssertionSyntaxListener`:L124
- `Analysis/TypeSystem/PilotErrorFlowAnalyzer.cs` (1240 L) — class `PilotErrorFlowAnalyzer`:L19, class `FallibleFunctionInfo`:L27, class `ErrorScope`:L45
- `Analysis/TypeSystem/PilotNullSafetyAnalyzer.cs` (1481 L) — class `PilotNullSafetyAnalyzer`:L16, class `FunctionSignatureInfo`:L18, class `TypeMemberInfo`:L26, class `NullSafetyScope`:L33
- `Analysis/TypeSystem/PilotType.cs` (277 L) — enum `PilotTypeKind`:L10, class `PilotType`:L32
- `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (573 L) — class `PilotAnalysisDiagnostic`:L13, class `PilotTypeInferenceEngine`:L39

### antlr
- `antlr/concurrency/ConcurrencyLexer.g4` (23 L)
- `antlr/concurrency/ConcurrencyParser.g4` (51 L)
- `antlr/core/CoreLexer.g4` (222 L)
- `antlr/core/CoreParser.g4` (730 L)
- `antlr/csharp/CSharpLexer.g4` (13 L)
- `antlr/csharp/CSharpParser.g4` (54 L)
- `antlr/database/DatabaseLexer.g4` (68 L)
- `antlr/database/DatabaseParser.g4` (163 L)
- `antlr/Pilot.g4` (1690 L)
- `antlr/realtime/RealtimeLexer.g4` (34 L)
- `antlr/realtime/RealtimeParser.g4` (32 L)
- `antlr/udsa/UDSALexer.g4` (19 L)
- `antlr/udsa/UDSAParser.g4` (37 L)

### Cli
- `Cli/PilotRepl.cs` (300 L) — class `PilotRepl`:L16
- `Cli/Program.cs` (1229 L) — class `Program`:L18

### CodeGen
- `CodeGen/Abstractions/IPilotCodeGenerator.cs` (20 L) — interface `IPilotCodeGenerator`:L9
- `CodeGen/Abstractions/PilotGeneratorContext.cs` (42 L) — class `PilotGeneratorContext`:L11
- `CodeGen/Abstractions/PilotGeneratorResult.cs` (31 L) — class `PilotGeneratorResult`:L10
- `CodeGen/Builtins/BuilderGenerator.cs` (52 L) — class `BuilderGenerator`:L13
- `CodeGen/Builtins/EqualityGenerator.cs` (58 L) — class `EqualityGenerator`:L13
- `CodeGen/Builtins/JsonGenerator.cs` (69 L) — class `JsonGenerator`:L13
- `CodeGen/Builtins/MetadataGenerator.cs` (58 L) — class `MetadataGenerator`:L13
- `CodeGen/Builtins/ObservableGenerator.cs` (70 L) — class `ObservableGenerator`:L17
- `CodeGen/Builtins/ReprGenerator.cs` (53 L) — class `ReprGenerator`:L13
- `CodeGen/Engine/PilotCodeGeneratorPipeline.cs` (410 L) — class `PilotCodeGeneratorPipeline`:L22
- `CodeGen/Model/ClassModel.cs` (74 L) — class `ClassModel`:L12
- `CodeGen/Model/DecoratorModel.cs` (37 L) — class `DecoratorModel`:L10
- `CodeGen/Model/MetadataModel.cs` (86 L) — class `MetadataModel`:L13
- `CodeGen/Model/MethodModel.cs` (33 L) — class `MethodModel`:L10
- `CodeGen/Model/ParameterModel.cs` (33 L) — class `ParameterModel`:L8
- `CodeGen/Model/PropertyModel.cs` (35 L) — class `PropertyModel`:L11
- `CodeGen/Model/ScriptModel.cs` (73 L) — class `ScriptModel`:L15

### Compiler
- `Compiler/IPilotCompiler.cs` (34 L) — interface `IPilotCompiler`:L8
- `Compiler/IPilotProgram.cs` (74 L) — interface `IPilotProgram`:L13
- `Compiler/PilotBytecodeCompiler.cs` (2464 L) — class `PilotBytecodeCompiler`:L17, class `LoopContext`:L225
- `Compiler/PilotCompilationException.cs` (19 L) — class `PilotCompilationException`:L8
- `Compiler/PilotCompilationResult.cs` (42 L) — class `PilotCompilationResult`:L9
- `Compiler/PilotCompiledProgram.cs` (607 L) — class `PilotCompiledProgram`:L20
- `Compiler/PilotCompiler.cs` (197 L) — class `PilotCompiler`:L17
- `Compiler/PilotCompilerOptions.cs` (110 L) — enum `CompilationTarget`:L9, class `PilotCompilerOptions`:L31
- `Compiler/PilotMatchBytecodeLowerer.cs` (2247 L) — class `PilotMatchBytecodeLowerer`:L18, class `StagedBinding`:L29, enum `ArmBranchKind`:L43, class `AnalyzedBranch`:L52
- `Compiler/PilotParserFast.cs` (83 L) — class `PilotParser`:L13

### Debugging
- `Debugging/DebuggerPauseEventArgs.cs` (26 L) — class `DebuggerPauseEventArgs`:L8
- `Debugging/DebuggerPauseReason.cs` (13 L) — enum `DebuggerPauseReason`:L6
- `Debugging/DebuggerState.cs` (11 L) — enum `DebuggerState`:L6
- `Debugging/DebuggerStepMode.cs` (13 L) — enum `DebuggerStepMode`:L6
- `Debugging/IPilotDebugger.cs` (144 L) — interface `IPilotDebugger`:L13
- `Debugging/PilotBreakpoint.cs` (86 L) — class `PilotBreakpoint`:L11
- `Debugging/PilotCallStack.cs` (74 L) — class `PilotCallStack`:L9
- `Debugging/PilotCliDebugger.cs` (307 L) — class `PilotCliDebugger`:L15
- `Debugging/PilotDebugger.cs` (291 L) — class `PilotDebugger`:L15
- `Debugging/PilotStackFrame.cs` (105 L) — class `PilotStackFrame`:L13

### Diagnostics
- `Diagnostics/ErrorTracker.cs` (31 L) — class `ErrorTracker`:L9
- `Diagnostics/ParseTreeVisualizer.cs` (33 L) — class `ParseTreeVisualizer`:L10
- `Diagnostics/PilotVersion.cs` (86 L) — class `PilotVersion`:L10

### docs
- `docs/concepts/01-types-variables-control-flow.md` (188 L)
- `docs/concepts/02-guard-defer-exceptions.md` (196 L)
- `docs/concepts/03-method-groups-and-dispatch.md` (155 L)
- `docs/concepts/04-type-adapters-and-repetition.md` (149 L)
- `docs/concepts/05-algebraic-classes-and-typestates.md` (188 L)
- `docs/concepts/06-projections-extensions-generative.md` (157 L)
- `docs/concepts/07-contextual-parameters-di.md` (136 L)
- `docs/concepts/08-exhaustive-pattern-matching.md` (172 L)
- `docs/concepts/09-design-by-contract.md` (179 L)
- `docs/concepts/10-database-and-sql.md` (181 L)
- `docs/concepts/11-concurrency-generators-lazy.md` (179 L)
- `docs/concepts/12-polyglot-interoperability.md` (155 L)
- `docs/concepts/13-metaprogramming-and-lisp.md` (173 L)
- `docs/concepts/14-realtime-and-matrix-types.md` (168 L)
- `docs/concepts/15-packaging-venv-and-tooling.md` (130 L)
- `docs/functions-and-apis.md` (401 L)
- `docs/index.md` (95 L)
- `docs/lexical-reference.md` (398 L)
- `docs/syntax-reference.md` (469 L)

### examples
- `examples/benchmarks/01_burst_vs_vm_benchmark.pilot` (91 L)
- `examples/benchmarks/01_burst_vs_vm_benchmark.s` (76 L)
- `examples/execution_demo.pilot` (50 L)
- `examples/mytest.pilot` (23 L)
- `examples/overview.pilot` (187 L)
- `examples/showcase/01_types_variables_control_flow.pilot` (98 L)
- `examples/showcase/02_guard_defer_exceptions.pilot` (91 L)
- `examples/showcase/03_method_groups_and_dispatch.pilot` (116 L)
- `examples/showcase/04_type_adapters_and_repetition.pilot` (96 L)
- `examples/showcase/05_algebraic_classes_and_typestates.pilot` (92 L)
- `examples/showcase/06_projections_lexical_extensions_generative.pilot` (81 L)
- `examples/showcase/07_contextual_parameters_di.pilot` (60 L)
- `examples/showcase/08_exhaustive_pattern_matching.pilot` (70 L)
- `examples/showcase/09_design_by_contract.pilot` (70 L)
- `examples/showcase/10_database_and_sql_blocks.pilot` (74 L)
- `examples/showcase/11_concurrency_generators_lazy.pilot` (98 L)
- `examples/showcase/11_source_generators_and_metaprogramming.pilot` (129 L)
- `examples/showcase/12_polyglot_interoperability.pilot` (86 L)
- `examples/showcase/12_selective_execution_and_testing.pilot` (81 L)
- `examples/showcase/13_metaprogramming_and_lisp.pilot` (66 L)
- `examples/showcase/14_realtime_and_matrix_types.pilot` (49 L)
- `examples/showcase/15_virtual_environment_and_packaging.pilot` (45 L)
- `examples/showcase/16_algebraic_data_types.pilot` (118 L)
- `examples/showcase/17_modern_null_safety.pilot` (33 L)
- `examples/showcase/18_values_first_error_system.pilot` (45 L)
- `examples/showcase/19_expression_ergonomics_and_extensions.pilot` (32 L)
- `examples/showcase/20_dynamic_for_expressions_and_c_abi.pilot` (46 L)
- `examples/showcase/run_all.ps1` (14 L)

### Native
- `Native/Assembly/PilotAssemblyEmitter.cs` (213 L) — class `PilotAssemblyEmitter`:L13
- `Native/CAbi/PilotCAbiEmitter.cs` (515 L) — class `PilotCAbiEmitter`:L19, class `StructDescriptor`:L190, class `AdtDescriptor`:L196, class `FunctionDescriptor`:L202, class `CAbiExtractionVisitor`:L209
- `Native/Llvm/PilotLlvmEmitter.cs` (1869 L) — class `PilotLlvmEmitter`:L21
- `Native/PilotNativeCompiler.cs` (373 L) — enum `NativeCompilationTarget`:L16, class `PilotNativeOptions`:L31, class `PilotNativeResult`:L46, class `PilotNativeCompiler`:L90
- `Native/Runtime/pilot_runtime.h` (164 L)
- `Native/Wasm/PilotWasmEmitter.cs` (878 L) — class `PilotWasmEmitter`:L18

### Pilot.LanguageServer
- `Pilot.LanguageServer/Lsp/PilotLspServer.cs` (747 L) — class `PilotLspServer`:L17
- `Pilot.LanguageServer/Pilot.LanguageServer.csproj` (15 L)
- `Pilot.LanguageServer/Program.cs` (53 L) — class `Program`:L10
- `Pilot.LanguageServer/Semantics/PilotDiagnosticCatalog.cs` (301 L) — class `PilotDiagnosticDescriptor`:L9, class `PilotDiagnosticCatalog`:L54
- `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (4641 L) — class `PilotSemanticAnalyzer`:L10, class `ScopeRestorer`:L4558, class `LspErrorListener`:L4570
- `Pilot.LanguageServer/Semantics/PilotSymbol.cs` (392 L) — enum `PilotSymbolKind`:L7, enum `PilotDiagnosticSeverity`:L35, class `PilotCodeAction`:L43, class `PilotDiagnostic`:L55, class `PilotSemanticTokenLegend`:L68, class `PilotSemanticToken`:L142, class `PilotSymbol`:L151, class `PilotSymbolReference`:L256, class `PilotScope`:L264, class `PilotAnalysisResult`:L310

### Pilot.Tests
- `Pilot.Tests/LspMessageHelper.cs` (13 L) — class `LspMessageHelper`:L6
- `Pilot.Tests/Pilot.Tests.csproj` (27 L)
- `Pilot.Tests/PilotAdtAndExhaustivenessTests.cs` (720 L) — class `PilotAdtAndExhaustivenessTests`:L33
- `Pilot.Tests/PilotAdvancedFeaturesTests.cs` (263 L) — class `PilotAdvancedFeaturesTests`:L11
- `Pilot.Tests/PilotBurstAndNativeTypesTests.cs` (150 L) — class `PilotBurstAndNativeTypesTests`:L14
- `Pilot.Tests/PilotCompilationAndScriptingTests.cs` (174 L) — class `PilotCompilationAndScriptingTests`:L11
- `Pilot.Tests/PilotComprehensiveKeywordValidationTests.cs` (452 L) — class `PilotComprehensiveKeywordValidationTests`:L13
- `Pilot.Tests/PilotContextContractsPatternsTests.cs` (297 L) — class `PilotContextContractsPatternsTests`:L11
- `Pilot.Tests/PilotContractDelegationAndStealingTests.cs` (234 L) — class `PilotContractDelegationAndStealingTests`:L10
- `Pilot.Tests/PilotDebuggingTests.cs` (265 L) — class `PilotDebuggingTests`:L12
- `Pilot.Tests/PilotDynamicForExpressionTests.cs` (345 L) — class `PilotDynamicForExpressionTests`:L27
- `Pilot.Tests/PilotErrorSystemTests.cs` (464 L) — class `PilotErrorSystemTests`:L22
- `Pilot.Tests/PilotExceptionProtectionsTests.cs` (196 L) — class `PilotExceptionProtectionsTests`:L11
- `Pilot.Tests/PilotExecutionEngineTests.cs` (260 L) — class `PilotExecutionEngineTests`:L9
- `Pilot.Tests/PilotExpressionErgonomicsTests.cs` (276 L) — class `PilotExpressionErgonomicsTests`:L16
- `Pilot.Tests/PilotFeedbackFeaturesTests.cs` (298 L) — class `PilotFeedbackFeaturesTests`:L11
- `Pilot.Tests/PilotHotReloadTests.cs` (159 L) — class `PilotHotReloadTests`:L11
- `Pilot.Tests/PilotIdeDiagnosticsTests.cs` (363 L) — class `PilotIdeDiagnosticsTests`:L10
- `Pilot.Tests/PilotLanguageTests.cs` (1541 L) — class `ParserHelper`:L9, class `ErrorTracker`:L36, class `TypeSafetyTests`:L52, class `DatabaseSetOperationsTests`:L125, class `ConcurrencyTests`:L178, class `RealTimeTests`:L256, class `UdsaMetaprogrammingTests`:L335, class `GuardDeferTests`:L398, class `PostgresDatabaseTests`:L465, class `CSharpRobustnessTests`:L571, class `ExtendedLanguageFeaturesTests`:L694, class `PublicClass`:L705, class `ProtectedClass`:L710, struct `InternalPoint`:L711, interface `IPublicService`:L716, enum `Status`:L720, class `MathUtils`:L947, class `PatternMatchingAndCollectionsTests`:L975, class `BuiltInTypesAndMatrixTests`:L1081, class `PreprocessingTests`:L1176, class `FileSuiteTests`:L1314, class `PolyglotInteropLanguageTests`:L1357, class `RootObjectAndLispMetaprogrammingTests`:L1451
- `Pilot.Tests/PilotMethodGroupTests.cs` (248 L) — class `PilotMethodGroupTests`:L11
- `Pilot.Tests/PilotNativeCompilerTests.cs` (290 L) — class `PilotNativeCompilerTests`:L16
- `Pilot.Tests/PilotNullSafetyTests.cs` (410 L) — class `PilotNullSafetyTests`:L25, class `SampleForeignApi`:L335
- `Pilot.Tests/PilotPackageManagerTests.cs` (116 L) — class `PilotPackageManagerTests`:L9
- `Pilot.Tests/PilotPolyglotExtendedTests.cs` (206 L) — class `PilotPolyglotExtendedTests`:L10
- `Pilot.Tests/PilotReplTests.cs` (110 L) — class `PilotReplTests`:L9
- `Pilot.Tests/PilotSelectiveExecutionTests.cs` (163 L) — class `PilotSelectiveExecutionTests`:L10
- `Pilot.Tests/PilotSemanticAndLspTests.cs` (897 L) — class `PilotSemanticAndLspTests`:L13, class `MathHelper`:L305
- `Pilot.Tests/PilotSourceGeneratorTests.cs` (208 L) — class `PilotSourceGeneratorTests`:L11
- `Pilot.Tests/PilotSyntaxAndLoopFixTests.cs` (110 L) — class `PilotSyntaxAndLoopFixTests`:L12
- `Pilot.Tests/PilotTranspilationAndMarshalingTests.cs` (225 L) — class `PilotTranspilationAndMarshalingTests`:L12, class `MathUtil`:L109, class `TestPerson`:L217
- `Pilot.Tests/PilotTypeAdapterAndRepetitionTests.cs` (308 L) — class `PilotTypeAdapterAndRepetitionTests`:L11
- `Pilot.Tests/PilotTypeInferenceAndBorrowCheckingTests.cs` (199 L) — class `PilotTypeInferenceAndBorrowCheckingTests`:L16
- `Pilot.Tests/PilotTypeInferenceAndHomogenizationTests.cs` (167 L) — class `PilotTypeInferenceAndHomogenizationTests`:L7
- `Pilot.Tests/PilotVersionTests.cs` (142 L) — class `PilotVersionTests`:L14
- `Pilot.Tests/PilotVirtualEnvironmentTests.cs` (118 L) — class `PilotVirtualEnvironmentTests`:L10
- `Pilot.Tests/PilotVirtualMachineTests.cs` (340 L) — class `PilotVirtualMachineTests`:L17

### Preprocessing
- `Preprocessing/PilotPreprocessor.cs` (983 L) — enum `PreprocessorDiagnosticSeverity`:L9, class `PreprocessorDiagnostic`:L16, class `CodeRegion`:L28, class `WarningSuppression`:L35, class `PilotPreprocessorOptions`:L42, class `PilotPreprocessResult`:L60, class `PilotPreprocessor`:L91, class `IfFrame`:L579, class `ConditionParser`:L649, class `ColonBlockFrame`:L755

### Runtime
- `Runtime/Adapters/PilotTypeAdapterRegistry.cs` (245 L) — class `PilotTypeAdapterRegistry`:L14, class `AdapterKeyComparer`:L230
- `Runtime/Builtins/IBuiltinFunction.cs` (15 L) — interface `IBuiltinFunction`:L10
- `Runtime/Builtins/PilotBuiltinRegistry.cs` (935 L) — class `PilotBuiltinRegistry`:L15
- `Runtime/Burst/PilotBurstEngine.cs` (145 L) — class `PilotBurstEngine`:L19
- `Runtime/Bytecode/PilotBytecodeChunk.cs` (586 L) — class `PilotJumpTable`:L12, class `PilotSerializedFieldDescriptor`:L29, class `PilotSerializedVariantDescriptor`:L38, class `PilotBytecodeTypeDescriptor`:L48, class `PilotBytecodeChunk`:L62
- `Runtime/Bytecode/PilotInstruction.cs` (46 L) — struct `PilotInstruction`:L11
- `Runtime/Bytecode/PilotOpcode.cs` (266 L) — enum `PilotOpcode`:L11
- `Runtime/Bytecode/PilVirtualMachine.cs` (1483 L) — class `PilVirtualMachine`:L20
- `Runtime/Concurrency/PilotFuture.cs` (72 L) — class `PilotFuture`:L11
- `Runtime/Contracts/PilotContractRegistry.cs` (313 L) — class `PilotContractRegistry`:L14
- `Runtime/ControlFlow/PilotControlFlowSignals.cs` (99 L) — enum `PilotFlowSignalKind`:L9, class `PilotControlFlowSignal`:L21, class `PilotReturnSignal`:L29, class `PilotBreakSignal`:L42, class `PilotContinueSignal`:L55, class `PilotYieldSignal`:L68, class `PilotRuntimeException`:L81
- `Runtime/ControlFlow/PilotExceptions.cs` (123 L) — class `PilotTypeException`:L9, class `PilotDivideByZeroException`:L18, class `PilotIndexOutOfRangeException`:L27, class `PilotKeyNotFoundException`:L36, class `PilotNullReferenceException`:L45, class `PilotAssertionException`:L54, class `PilotRecursionException`:L63, class `PilotContractException`:L72, class `PilotTimeoutException`:L81, class `PilotPatternMatchingException`:L90, class `PilotContextException`:L99, class `PilotUserException`:L108
- `Runtime/Environment/PilotBinding.cs` (22 L) — class `PilotBinding`:L8
- `Runtime/Environment/PilotEnvironment.cs` (59 L) — class `PilotEnvironment`:L9
- `Runtime/Environment/PilotScope.cs` (137 L) — class `PilotScope`:L10
- `Runtime/Execution/IPilotEngine.cs` (56 L) — interface `IPilotEngine`:L10
- `Runtime/Execution/PilotEngine.cs` (650 L) — class `PilotEngine`:L19, class `GeneratorRunner`:L351, class `MergedTextWriter`:L614
- `Runtime/Execution/PilotExecutionContext.cs` (153 L) — class `PilotExecutionContext`:L13
- `Runtime/Execution/PilotExecutionOptions.cs` (87 L) — class `PilotExecutionOptions`:L13
- `Runtime/Execution/PilotExecutionResult.cs` (56 L) — class `PilotExecutionResult`:L10
- `Runtime/Execution/PilotInterpreterVisitor.cs` (5145 L) — class `PilotInterpreterVisitor`:L19, struct `FStringSegment`:L41, enum `TargetAssignmentKind`:L57, class `TargetAssignmentInfo`:L66
- `Runtime/Execution/PilotPatternMatcher.cs` (970 L) — class `PilotPatternMatcher`:L16
- `Runtime/Extensions/PilotLexicalExtension.cs` (138 L) — class `PilotLexicalExtension`:L11
- `Runtime/Generative/PilotClassNode.cs` (127 L) — class `PilotFieldNode`:L11, class `PilotClassNode`:L38
- `Runtime/HotReload/PilotHotReloader.cs` (732 L) — class `PilotSymbolSnapshot`:L19, class `PilotPatchRecord`:L165, class `PilotPatchResult`:L208, class `PilotHotReloadSession`:L242
- `Runtime/Interop/IPolyglotProvider.cs` (31 L) — interface `IPolyglotProvider`:L10
- `Runtime/Interop/IPolyglotTypeConverter.cs` (20 L) — interface `IPolyglotTypeConverter`:L9
- `Runtime/Interop/PolyglotRegistry.cs` (670 L) — class `PolyglotRegistry`:L12, class `DefaultPythonProvider`:L72, class `DefaultCSharpProvider`:L149, class `DefaultRustProvider`:L221, class `DefaultJavaScriptProvider`:L264, class `DefaultJavaProvider`:L318, class `DefaultAsmProvider`:L370, class `DefaultCProvider`:L439, class `DefaultCppProvider`:L484, class `DefaultLuaProvider`:L528, class `DefaultPostgreSqlProvider`:L578, class `DefaultMySqlProvider`:L652
- `Runtime/Interop/PolyglotTypeBridge.cs` (442 L) — class `PolyglotTypeBridge`:L16
- `Runtime/NativeMemory/PilotLoopAbiTypes.cs` (122 L) — struct `RangeWindow`:L15, struct `SliceBuffer`:L48
- `Runtime/NativeMemory/PilotNativeArray.cs` (212 L) — class `PilotNativeArray`:L15
- `Runtime/NativeMemory/PilotNativeMap.cs` (281 L) — class `PilotNativeMap`:L15
- `Runtime/Packaging/Adapters/BaseInteropPackageAdapter.cs` (218 L) — class `BaseInteropPackageAdapter`:L12, class `NuGetPackageAdapter`:L140, class `NpmPackageAdapter`:L156, class `PipPackageAdapter`:L171, class `CargoPackageAdapter`:L186, class `MavenPackageAdapter`:L200, class `LuaRocksPackageAdapter`:L205
- `Runtime/Packaging/IInteropPackageAdapter.cs` (31 L) — interface `IInteropPackageAdapter`:L10
- `Runtime/Packaging/PilotPackageLock.cs` (65 L) — class `PilotPackageLock`:L12, class `LockedPackageEntry`:L49
- `Runtime/Packaging/PilotPackageManager.cs` (376 L) — class `PilotPackageManager`:L17
- `Runtime/Packaging/PilotPackageManifest.cs` (100 L) — class `PilotPackageManifest`:L14
- `Runtime/SelectiveExecution/PilotBenchmarkReport.cs` (47 L) — class `PilotBenchmarkResult`:L9, class `PilotBenchmarkReport`:L37
- `Runtime/SelectiveExecution/PilotSelectiveExecutionEngine.cs` (323 L) — class `PilotSelectiveExecutionEngine`:L20
- `Runtime/SelectiveExecution/PilotTestReport.cs` (46 L) — class `PilotTestCaseResult`:L9, class `PilotTestReport`:L32
- `Runtime/Serialization/PilotJsonSerializer.cs` (198 L) — class `PilotJsonSerializer`:L15
- `Runtime/Values/PilotAdt.cs` (1037 L) — class `PilotAdtFieldDefinition`:L14, record `PilotFieldMemoryLayout`:L35, record `PilotVariantMemoryLayout`:L46, class `PilotAdtMemoryLayout`:L58, class `PilotAdtFloatSlot`:L563, class `PilotAdtVariantDefinition`:L572, class `PilotAdtDefinition`:L721, class `PilotAdtInstance`:L768, class `PilotRecordDefinition`:L828, class `PilotRecordInstance`:L951, class `PilotKeywordVariant`:L1002
- `Runtime/Values/PilotFunction.cs` (270 L) — class `PilotParameter`:L12, class `PilotFunction`:L37
- `Runtime/Values/PilotGenerator.cs` (123 L) — enum `PilotGeneratorState`:L13, class `PilotGenerator`:L25
- `Runtime/Values/PilotKeyword.cs` (96 L) — class `PilotKeyword`:L12
- `Runtime/Values/PilotLazy.cs` (55 L) — class `PilotLazy`:L9
- `Runtime/Values/PilotMatrix.cs` (209 L) — class `PilotMatrix`:L9
- `Runtime/Values/PilotMethodGroup.cs` (551 L) — class `PilotMethodGroup`:L20
- `Runtime/Values/PilotObject.cs` (649 L) — class `PilotClassDefinition`:L11, class `PilotObject`:L393
- `Runtime/Values/PilotRange.cs` (98 L) — class `PilotRange`:L10
- `Runtime/Values/PilotValue.cs` (2103 L) — struct `PilotValue`:L15, class `PilotNestedSomeSentinel`:L27, class `PilotFailurePrimitiveSentinel`:L32, class `PilotNestedFailureBox`:L47
- `Runtime/Values/PilotValueKind.cs` (38 L) — enum `PilotValueKind`:L6
- `Runtime/Venv/PilotVirtualEnvironment.cs` (446 L) — class `PilotVenvOptions`:L12, class `PilotVenvConfig`:L23, class `PilotVirtualEnvironment`:L116

### Scripting
- `Scripting/PilotScript.cs` (123 L) — class `PilotScript`:L13
- `Scripting/PilotScriptContext.cs` (183 L) — class `PilotScriptContext`:L19
- `Scripting/PilotScriptExitException.cs` (20 L) — class `PilotScriptExitException`:L8

### Testing
- `Testing/PilotTestRunner.cs` (104 L) — class `PilotTestRunner`:L13

### tests
- `tests/01_type_safety.pilot` (108 L)
- `tests/02_database_set_operations.pilot` (84 L)
- `tests/03_concurrency.pilot` (89 L)
- `tests/04_realtime.pilot` (68 L)
- `tests/05_udsa_metaprogramming.pilot` (60 L)
- `tests/06_syntax_edge_cases.pilot` (66 L)
- `tests/07_syntax_errors.pilot` (10 L)
- `tests/08_guard_defer.pilot` (100 L)
- `tests/09_postgres_database.pilot` (119 L)
- `tests/10_csharp_robustness.pilot` (128 L)
- `tests/11_modern_language_features.pilot` (253 L)
- `tests/12_pattern_matching_collections.pilot` (162 L)
- `tests/13_builtin_types_matrix.pilot` (86 L)
- `tests/14_preprocessing_directives.pilot` (67 L)
- `tests/15_polyglot_interop.pilot` (91 L)
- `tests/16_root_object_and_lisp_metaprogramming.pilot` (144 L)
- `tests/17_advanced_runtime_features.pilot` (108 L)
- `tests/18_contracts_and_method_stealing.pilot` (126 L)
- `tests/19_exception_protections.pilot` (80 L)
- `tests/20_compilation_and_scripting.pilot` (38 L)
- `tests/21_debugging_capabilities.pilot` (34 L)
- `tests/22_method_groups_and_dispatch.pilot` (151 L)
- `tests/23_algebraic_typestates_generative.pilot` (119 L)
- `tests/24_contextual_contracts_pattern_matching.pilot` (102 L)
- `tests/25_polyglot_extended_interop.pilot` (102 L)
- `tests/26_type_adapters_and_repetition.pilot` (80 L)
- `tests/27_virtual_environment.pilot` (30 L)
- `tests/28_adt_and_exhaustive_matching.pilot` (146 L)
- `tests/29_null_safety_and_optionals.pilot` (114 L)
- `tests/30_values_first_errors.pilot` (109 L)
- `tests/31_expression_ergonomics_and_extensions.pilot` (154 L)
- `tests/32_dynamic_for_expressions_and_c_abi.pilot` (69 L)

### Transpilation
- `Transpilation/ITargetLanguageConverter.cs` (13 L) — interface `ITargetLanguageConverter`:L7
- `Transpilation/ITargetLanguageIngestor.cs` (17 L) — interface `ITargetLanguageIngestor`:L6
- `Transpilation/ITargetLanguageTranspiler.cs` (22 L) — interface `ITargetLanguageTranspiler`:L6
- `Transpilation/PilotLibraryMapper.cs` (524 L) — class `PilotLibraryMapper`:L19
- `Transpilation/PilotLibraryTranspiler.cs` (262 L) — class `LibraryTranspilationSummary`:L12, class `PilotLibraryTranspiler`:L27
- `Transpilation/PilotOptimizer.cs` (398 L) — class `PilotOptimizer`:L15
- `Transpilation/Targets/CppTargetConverter.cs` (581 L) — class `CppTargetConverter`:L17, class `PilotToCppVisitor`:L232
- `Transpilation/Targets/CSharpTargetConverter.cs` (422 L) — class `CSharpTargetConverter`:L14, class `PilotToCSharpVisitor`:L152
- `Transpilation/Targets/CTargetConverter.cs` (577 L) — class `CTargetConverter`:L17, class `PilotToCVisitor`:L205
- `Transpilation/Targets/JavaScriptTargetConverter.cs` (495 L) — class `JavaScriptTargetConverter`:L16, class `PilotToJsVisitor`:L161
- `Transpilation/Targets/LuaTargetConverter.cs` (528 L) — class `LuaTargetConverter`:L16, class `PilotToLuaVisitor`:L181
- `Transpilation/Targets/PythonTargetConverter.cs` (531 L) — class `PythonTargetConverter`:L15, class `PilotToPythonVisitor`:L172
- `Transpilation/Targets/RustTargetConverter.cs` (479 L) — class `RustTargetConverter`:L17, class `PilotToRustVisitor`:L137
- `Transpilation/TranspilationOptions.cs` (51 L) — class `TranspilationOptions`:L9
- `Transpilation/TranspilationRegistry.cs` (149 L) — class `TranspilationRegistry`:L12
- `Transpilation/TranspilationResult.cs` (36 L) — class `TranspilationResult`:L9

### vscode-pilot
- `vscode-pilot/language-configuration.json` (38 L)
- `vscode-pilot/package-lock.json` (419 L)
- `vscode-pilot/package.json` (197 L)
- `vscode-pilot/snippets/pilot.json` (342 L)
- `vscode-pilot/src/extension.ts` (700 L)
- `vscode-pilot/syntaxes/pilot.tmLanguage.json` (401 L)
- `vscode-pilot/test/01_types_variables_control_flow.pilot` (98 L)
- `vscode-pilot/test/02_guard_defer_exceptions.pilot` (91 L)
- `vscode-pilot/test/03_method_groups_and_dispatch.pilot` (116 L)
- `vscode-pilot/test/04_type_adapters_and_repetition.pilot` (96 L)
- `vscode-pilot/test/05_algebraic_classes_and_typestates.pilot` (92 L)
- `vscode-pilot/test/06_projections_lexical_extensions_generative.pilot` (81 L)
- `vscode-pilot/test/07_contextual_parameters_di.pilot` (60 L)
- `vscode-pilot/test/08_exhaustive_pattern_matching.pilot` (70 L)
- `vscode-pilot/test/09_design_by_contract.pilot` (70 L)
- `vscode-pilot/test/10_database_and_sql_blocks.pilot` (74 L)
- `vscode-pilot/test/11_concurrency_generators_lazy.pilot` (98 L)
- `vscode-pilot/test/12_polyglot_interoperability.pilot` (86 L)
- `vscode-pilot/test/13_metaprogramming_and_lisp.pilot` (66 L)
- `vscode-pilot/test/14_realtime_and_matrix_types.pilot` (49 L)
- `vscode-pilot/test/15_virtual_environment_and_packaging.pilot` (45 L)
- `vscode-pilot/tsconfig.json` (13 L)
