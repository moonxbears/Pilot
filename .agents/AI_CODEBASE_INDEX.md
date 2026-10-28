# Pilot — AI-Only Codebase Navigation & Symbol Index

> **STRICTLY FOR AI AGENTS**: Do not link this file in user-facing documentation. Use exact paths and `:L<line>` anchors below with `read_file` (`StartLine`/`EndLine`) to jump directly to code without exploratory directory/grep bouncing.
> **Auto-Sync Stamp**: `319F04E20E9130AB3B83BCFE34A448B1FD2BA5D3DBBA5C417877B7416D131FC9` | **Total Indexed Files**: 316 | **Regenerate**: `pwsh -NoProfile -File .agents/scripts/update_ai_index.ps1`

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
| **Static Analysis, Type Inference & Call-Quoting** | `Analysis/PilotAnalysisPipeline.cs`, `Analysis/TypeSystem/PilotTypeInferenceEngine.cs`, `PilotBidirectionalInferencer.cs`, `PilotType.cs`, `Analysis/CallQuoting/PilotCallQuotingEngine.cs`, `Analysis/Autodiff/PilotSymbolicDifferentiator.cs`, `Runtime/Reactive/PilotSignalGraph.cs`, `Runtime/Effects/PilotEffectDryRunEngine.cs` | Orchestrates bidirectional/HM inference, Tarjan SCC fixed-point, flow narrowing, row polymorphism, call-quoting modes 0–7 (`!`, `!!`, `~`, `!&`, `!^`, `!@`, `!?`), exhaustiveness, borrow checking, dimensions, and contracts. |
| **Exhaustive Pattern Matching** | `Analysis/PatternMatching/PilotExhaustivenessChecker.cs`, `Runtime/Execution/PilotPatternMatcher.cs` | Compile-time usefulness/exhaustiveness checking + runtime destructuring. |
| **Ownership, Loops, Units & Contracts** | `Analysis/Ownership/PilotBorrowChecker.cs`, `Analysis/Loops/PilotForLoopAnalyzer.cs`, `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs`, `Analysis/ContractVerification/PilotContractVerifier.cs` | Runtime contracts registry in `Runtime/Contracts/PilotContractRegistry.cs`; loop ABI types in `Runtime/NativeMemory/PilotLoopAbiTypes.cs`. |
| **Built-in Functions & Host Scripting** | `Runtime/Builtins/PilotBuiltinRegistry.cs`, `Scripting/PilotScriptContext.cs`, `Scripting/PilotScript.cs` | Global intrinsics (`println`, `len`, `json`, `venv`, `matrix`, file/OS helpers). |
| **Type Adapters & Extensions** | `Runtime/Adapters/PilotTypeAdapterRegistry.cs`, `Runtime/Extensions/PilotLexicalExtension.cs` | First-class `adapter` conversion chains & `extension ... on ...` dispatch. |
| **Compile-Time `@` Code Generators** | `CodeGen/Engine/PilotCodeGeneratorPipeline.cs`, `CodeGen/Builtins/*.cs` | `@builder`, `@equality`, `@json`, `@metadata`, `@observable`, `@repr`. |
| **Bidirectional Source Transpilers** | `Transpilation/TranspilationRegistry.cs`, `PilotLibraryTranspiler.cs`, `PilotLibraryMapper.cs`, `Transpilation/Targets/*` | `--emit <target>` and `--from <lang>` for C, C++, C#, Rust, Python, JS, Lua + `PilotOptimizer.cs`. |
| **Native AOT & Zero-Copy Shared Memory** | `Native/PilotNativeCompiler.cs`, `Native/Llvm/PilotLlvmEmitter.cs`, `Native/Wasm/PilotWasmEmitter.cs`, `Native/CAbi/PilotCAbiEmitter.cs`, `Native/Assembly/PilotAssemblyEmitter.cs`, `Runtime/NativeMemory/PilotSharedBuffer.cs`, `Runtime/NativeMemory/PilotLoopAbiTypes.cs` | Native C header in `Native/Runtime/pilot_runtime.h`; 24-byte C-ABI `SliceBuffer<T>` / `mmap` IPC in `PilotSharedBuffer.cs`; Burst in `Runtime/Burst/PilotBurstEngine.cs`. |
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

Total lines in `antlr/Pilot.g4`: **1712**

### Parser Rules (237)
`program`:L15 · `identifier`:L23 · `stmt`:L53 · `simple_stmt`:L58 · `small_stmt`:L62 · `compound_stmt`:L90
`suite`:L141 · `guard_stmt`:L151 · `pattern_guard`:L155 · `defer_stmt`:L160 · `type_annotation`:L168
`return_type`:L172 · `type_spec`:L176 · `error_union_type`:L180 · `error_primary_type`:L184 · `union_type`:L191
`intersection_type`:L195 · `primary_type`:L199 · `base_type`:L203 · `type_arguments`:L214 · `generic_params`:L218
`generic_param`:L222 · `where_type_clause`:L227 · `where_type_item`:L231 · `access_modifier`:L236
`declaration_modifier`:L243 · `binding_pattern`:L253 · `tuple_pattern`:L261 · `list_pattern`:L265
`record_pattern`:L269 · `binding_pattern_item`:L273 · `record_pattern_field`:L277 · `pattern_assign_stmt`:L282
`let_stmt`:L286 · `var_stmt`:L290 · `const_stmt`:L294 · `ref_stmt`:L298 · `dynamic_stmt`:L302
`type_alias_stmt`:L306 · `adt_sum_body`:L311 · `adt_variant`:L317 · `adt_variant_payload`:L321
`adt_field_list`:L325 · `adt_field`:L329 · `annassign_stmt`:L334 · `expr_stmt`:L338 · `channel_send_stmt`:L342
`augassign`:L346 · `func_def`:L366 · `contract_clause`:L370 · `parameters`:L375 · `parameter`:L379
`class_def`:L383 · `state_def`:L388 · `class_bases`:L392 · `struct_def`:L397 · `record_def`:L402
`struct_member`:L407 · `interface_def`:L412 · `enum_def`:L417 · `enum_suite`:L421 · `enum_member`:L425
`actor_def`:L430 · `model_def`:L435 · `property_def`:L444 · `property_body`:L448 · `property_accessor`:L452
`get_accessor`:L457 · `set_accessor`:L461 · `operator_def`:L466 · `overloadable_op`:L470 · `extension_def`:L478
`adapter_def`:L484 · `macro_def`:L489 · `quote_stmt`:L493 · `quote_expr`:L497 · `quasiquote_expr`:L501
`splice_expr`:L505 · `extern_def`:L512 · `extern_member`:L516 · `foreign_language`:L522
`inline_foreign_block`:L526 · `decorators`:L531 · `decorator`:L535 · `labeled_stmt`:L543 · `block`:L547
`if_stmt`:L551 · `while_stmt`:L555 · `for_stmt`:L559 · `for_target`:L563 · `match_stmt`:L568 · `match_suite`:L573
`match_arm`:L577 · `pattern`:L581 · `or_pattern`:L585 · `and_pattern`:L589 · `not_pattern`:L593
`primary_pattern`:L599 · `range_pattern`:L612 · `range_bound`:L616 · `signed_number_pattern`:L621
`keyword_pattern`:L625 · `pattern_arg`:L629 · `relational_pattern`:L634 · `collection_pattern`:L638
`slice_pattern_element`:L642 · `property_pattern`:L647 · `property_pattern_member`:L651
`tuple_pattern_match`:L655 · `type_or_var_pattern`:L659 · `generic_pattern_suffix`:L664 · `try_stmt`:L668
`except_clause`:L672 · `with_stmt`:L676 · `with_item`:L680 · `with_context_binding`:L687 · `flow_stmt`:L691
`pass_stmt`:L699 · `assert_stmt`:L703 · `steal_stmt`:L707 · `delegate_stmt`:L711 · `contract_forward_stmt`:L715
`global_stmt`:L719 · `import_stmt`:L723 · `foreign_import_stmt`:L729 · `foreign_target`:L734 · `import_name`:L738
`import_from`:L742 · `import_as_name`:L746 · `dotted_as_name`:L750 · `import_as_names`:L754
`dotted_as_names`:L758 · `dotted_name`:L762 · `deadline_stmt`:L770 · `within_stmt`:L774 · `periodic_stmt`:L778
`noalloc_stmt`:L782 · `region_stmt`:L786 · `duration_term`:L790 · `spawn_stmt`:L799 · `atomic_stmt`:L803
`sync_stmt`:L807 · `isolate_stmt`:L811 · `select_stmt`:L815 · `select_suite`:L819 · `select_arm`:L823
`select_case_target`:L829 · `dsl_stmt`:L839 · `dsl_args`:L843 · `unit_stmt`:L847 · `unit_expr`:L851
`unit_term`:L855 · `database_stmt`:L866 · `database_def`:L870 · `transaction_stmt`:L875 · `savepoint_stmt`:L879
`savepoint_block`:L883 · `commit_stmt`:L887 · `rollback_stmt`:L891 · `insert_stmt`:L896
`insert_values_group`:L903 · `on_conflict_clause`:L907 · `update_stmt`:L912 · `set_assignment_list`:L919
`set_assignment`:L923 · `delete_stmt`:L928 · `returning_clause`:L934 · `index_clause`:L939 · `index_field`:L943
`index_type`:L947 · `index_hint`:L951 · `query_expr`:L956 · `from_clause`:L960 · `join_clause`:L964
`where_clause`:L968 · `group_clause`:L972 · `order_clause`:L976 · `ordering_term`:L980 · `select_clause`:L984
`select_projection`:L988 · `select_item`:L993 · `limit_clause`:L997 · `sql_query_expr`:L1002 · `from_source`:L1006
`lookup_expr`:L1011 · `set_op`:L1016 · `testlist`:L1027 · `exprlist`:L1031 · `test`:L1035 · `lambda_expr`:L1040
`lambda_body`:L1049 · `capture_list`:L1054 · `capture_item`:L1058 · `lambda_params`:L1064 · `lambda_param`:L1069
`pipe_expr`:L1074 · `set_query_expr`:L1079 · `coalesce_expr`:L1086 · `catch_clause`:L1090 · `or_test`:L1096
`and_test`:L1100 · `not_test`:L1104 · `comparison`:L1109 · `comp_op`:L1113 · `range_expr`:L1126
`bitor_expr`:L1130 · `bitxor_expr`:L1134 · `bitand_expr`:L1138 · `shift_expr`:L1142 · `arith_expr`:L1146
`term`:L1150 · `factor`:L1154 · `unary_op`:L1160 · `power`:L1170 · `atom_expr`:L1174 · `trailer`:L1178
`call_quote_op`:L1195 · `cascade_trailer`:L1204 · `cascade_target`:L1208 · `over_clause`:L1218
`subscriptlist`:L1222 · `subscript`:L1226 · `atom`:L1234 · `collection_expr`:L1263 · `collection_element`:L1267
`nameof_expr`:L1271 · `macro_call`:L1275 · `symbol_literal`:L1279 · `literal`:L1285 · `testlist_comp`:L1304
`dictorsetmaker`:L1308 · `dict_entry`:L1313 · `comp_for`:L1318 · `comp_iter`:L1322 · `comp_if`:L1327
`arglist`:L1331 · `argument`:L1335

### Lexer Tokens (262)
`PUBLIC`:L1349 · `PRIVATE`:L1350 · `PROTECTED`:L1351 · `INTERNAL`:L1352 · `STATIC`:L1353 · `PARTIAL`:L1354
`STEAL`:L1355 · `CONTRACTS`:L1356 · `DELEGATE`:L1357 · `PARAMS`:L1358 · `ARGS`:L1359 · `KWARGS`:L1360
`NAMEOF`:L1361 · `EACH`:L1362 · `REPEAT`:L1363 · `ADAPTER`:L1364 · `CONVERTER`:L1365 · `DEF`:L1367 · `FN`:L1368
`CLASS`:L1369 · `STRUCT`:L1370 · `RECORD`:L1371 · `INTERFACE`:L1372 · `PROTOCOL`:L1373 · `ENUM`:L1374
`ACTOR`:L1375 · `LET`:L1376 · `VAR`:L1377 · `CONST`:L1378 · `TYPE`:L1379 · `REF`:L1380 · `DYNAMIC`:L1381
`SYMBOL`:L1382 · `PURE`:L1383 · `MUT`:L1384 · `IMMUT`:L1385 · `PROPERTY`:L1386 · `GET`:L1387 · `OPERATOR`:L1388
`EXTENSION`:L1389 · `EXTEND`:L1390 · `STATE`:L1391 · `SELECT`:L1394 · `FROM`:L1395 · `WHERE`:L1396 · `JOIN`:L1397
`INNER`:L1398 · `LEFT`:L1399 · `RIGHT`:L1400 · `FULL`:L1401 · `CROSS`:L1402 · `OUTER`:L1403 · `ON`:L1404
`GROUP`:L1405 · `BY`:L1406 · `HAVING`:L1407 · `ORDER`:L1408 · `ASC`:L1409 · `DESC`:L1410 · `ASCENDING`:L1411
`DESCENDING`:L1412 · `LIMIT`:L1413 · `OFFSET`:L1414 · `INTO`:L1415 · `DISTINCT`:L1416 · `UNION`:L1417 · `ALL`:L1418
`INTERSECT`:L1419 · `EXCEPT`:L1420 · `DIFFERENCE`:L1421 · `INDEX`:L1422 · `INDEXED`:L1423 · `LOOKUP`:L1424
`TABLE`:L1425 · `SCHEMA`:L1426 · `INSERT`:L1427 · `UPDATE`:L1428 · `DELETE`:L1429 · `VALUES`:L1430 · `SET_KW`:L1431
`RETURNING`:L1432 · `CONFLICT`:L1433 · `DO`:L1434 · `NOTHING`:L1435 · `OVER`:L1436 · `PARTITION`:L1437
`WINDOW`:L1438 · `TRANSACTION`:L1439 · `COMMIT`:L1440 · `ROLLBACK`:L1441 · `SAVEPOINT`:L1442 · `TO`:L1443
`CONNECT`:L1444 · `DATABASE`:L1445 · `MODEL`:L1446 · `ASYNC`:L1449 · `AWAIT`:L1450 · `SPAWN`:L1451
`PARALLEL`:L1452 · `CHAN`:L1453 · `CHANNEL`:L1454 · `ATOMIC`:L1455 · `SYNC`:L1456 · `LOCK`:L1457 · `ISOLATE`:L1458
`REALTIME`:L1461 · `DEADLINE`:L1462 · `ON_MISSED`:L1463 · `WITHIN`:L1464 · `FALLBACK`:L1465 · `PERIOD`:L1466
`PERIODIC`:L1467 · `PRIORITY`:L1468 · `JITTER`:L1469 · `BOUNDED`:L1470 · `NOALLOC`:L1471 · `STACKALLOC`:L1472
`REGION`:L1473 · `WCET`:L1474 · `HARD_RT`:L1475 · `SOFT_RT`:L1476 · `EXTERN`:L1479 · `CSHARP`:L1480
`PYTHON`:L1481 · `RUST`:L1482 · `JS`:L1483 · `JAVASCRIPT`:L1484 · `JAVA`:L1485 · `ASM`:L1486 · `ASSEMBLY`:L1487
`CPP`:L1488 · `C_KW`:L1489 · `LUA`:L1490 · `SQL_KW`:L1491 · `POSTGRES`:L1492 · `POSTGRESQL`:L1493 · `MYSQL`:L1494
`MODULE`:L1495 · `LIB`:L1496 · `DSL`:L1499 · `DOMAIN`:L1500 · `UNIT`:L1501 · `MACRO`:L1502 · `DEFMACRO`:L1503
`DEFKEYWORD`:L1504 · `QUOTE`:L1505 · `QUASIQUOTE`:L1506 · `SPLICE`:L1507 · `UNQUOTE`:L1508
`UNQUOTE_SPLICING`:L1509 · `MATCH`:L1510 · `CASE`:L1511 · `WHEN`:L1512 · `DEFAULT`:L1513 · `TIMEOUT`:L1514
`GUARD`:L1517 · `DEFER`:L1518 · `CONTEXT`:L1519 · `REQUIRES`:L1520 · `ENSURES`:L1521 · `IF`:L1522 · `ELIF`:L1523
`ELSE`:L1524 · `THEN`:L1525 · `WHILE`:L1526 · `FOR`:L1527 · `IN`:L1528 · `RETURN`:L1529 · `YIELD`:L1530
`BREAK`:L1531 · `CONTINUE`:L1532 · `PASS`:L1533 · `RAISE`:L1534 · `TRY`:L1535 · `CATCH`:L1536 · `FINALLY`:L1537
`WITH`:L1538 · `AS`:L1539 · `ASSERT`:L1540 · `IMPORT`:L1541 · `GLOBAL`:L1542 · `LAMBDA`:L1543 · `AND`:L1544
`OR`:L1545 · `NOT`:L1546 · `IS`:L1547 · `TRUE`:L1550 · `FALSE`:L1551 · `SOME`:L1552 · `NONE`:L1553 · `NULL`:L1554
`UNDEFINED`:L1555 · `JSON_GET_TEXT`:L1558 · `ARROW_LEFT`:L1559 · `ARROW`:L1560 · `FAT_ARROW`:L1561
`PIPE_FORWARD`:L1562 · `PIPE_BACKWARD`:L1563 · `SAFE_CASCADE`:L1564 · `SAFE_NAV`:L1565 · `COALESCE_EQUAL`:L1566
`COALESCE`:L1567 · `JSON_CONTAINS`:L1568 · `JSON_CONTAINED`:L1569 · `ELLIPSIS`:L1570 · `DOTDOT_EQUAL`:L1571
`DOTDOT_LESS`:L1572 · `DOTDOT`:L1573 · `DOUBLESTAR`:L1574 · `DOUBLESLASH`:L1575 · `CALL_QUOTE_DEEP`:L1576
`CALL_QUOTE_ABI`:L1577 · `CALL_QUOTE_REACT`:L1578 · `CALL_QUOTE_ADJOINT`:L1579 · `CALL_QUOTE_EFFECT`:L1580
`EQEQUAL`:L1581 · `NOTEQUAL`:L1582 · `LESSEQUAL`:L1583 · `GREATEREQUAL`:L1584 · `LEFTSHIFT`:L1585
`RIGHTSHIFT`:L1586 · `PLUSEQUAL`:L1587 · `MINEQUAL`:L1588 · `STAREQUAL`:L1589 · `SLASHEQUAL`:L1590
`PERCENTEQUAL`:L1591 · `AMPEREQUAL`:L1592 · `VBAREQUAL`:L1593 · `CIRCUMFLEXEQUAL`:L1594 · `LEFTSHIFTEQUAL`:L1595
`RIGHTSHIFTEQUAL`:L1596 · `DOUBLESTAREQUAL`:L1597 · `DOUBLESLASHEQUAL`:L1598 · `WALRUS`:L1599 · `LPAR`:L1602
`RPAR`:L1603 · `LSQB`:L1604 · `RSQB`:L1605 · `LBRACE`:L1606 · `RBRACE`:L1607 · `COLON`:L1608 · `SEMI`:L1609
`COMMA`:L1610 · `DOT`:L1611 · `AT`:L1612 · `PLUS`:L1613 · `MINUS`:L1614 · `STAR`:L1615 · `SLASH`:L1616
`PERCENT`:L1617 · `VBAR`:L1618 · `AMPER`:L1619 · `CIRCUMFLEX`:L1620 · `TILDE`:L1621 · `LESS`:L1622
`GREATER`:L1623 · `EQUAL`:L1624 · `QUESTION`:L1625 · `EXCLAMATION`:L1626 · `DOLLAR`:L1627 · `BACKQUOTE`:L1628
`UNDERSCORE`:L1629

---

## 3. Large File Internal Method Maps (Files > 350 Lines)

Use these `:L<line>` method anchors to pass exact `StartLine`/`EndLine` into `read_file` for large files:

### `Analysis/Autodiff/PilotSymbolicDifferentiator.cs` (759 lines)
`Equals`:L33 · `Equals`:L36 · `GetHashCode`:L38 · `ToString`:L40 · `PilotCseBinding`:L48 · `ToString`:L53
`ToDualComputeImpulseStruct`:L105 · `GetTupleElements`:L118 · `ToTupleValue`:L125 · `ToPilotObject`:L127
`ValidateDifferentiableBody`:L193 · `ScanTreeForNonDifferentiableConstructs`:L204 · `EvaluateSymbolicAdjoint`:L272
`Differentiate`:L403 · `RewriteChildrenWithCse`:L620 · `RewriteTreeWithCse`:L646
`EvaluateSymbolicAstWithEnv`:L660

### `Analysis/CallQuoting/PilotCallQuotingEngine.cs` (1661 lines)
`Invoke`:L67 · `ToPilotObject`:L80 · `ToSExpr`:L116 · `ToString`:L118 · `Const`:L186 · `Sym`:L189 · `Binary`:L192
`Add`:L195 · `Sub`:L196 · `Mul`:L197 · `Div`:L198 · `Unary`:L200 · `If`:L203 · `Call`:L206 · `Let`:L209
`GetFreeSymbols`:L215 · `CollectSymbols`:L222 · `FoldConstants`:L260 · `FoldCore`:L269 · `SubstituteSymbol`:L400
`Evaluate`:L437 · `ToPilotObject`:L494 · `ToInfixString`:L555 · `ToSExpr`:L570 · `ToString`:L572
`FormatScalarValue`:L631 · `IsZeroConst`:L649 · `IsOneConst`:L654 · `TryEvaluateBinaryConst`:L659
`InferValueTypeName`:L689 · `AsStruct2x32`:L763 · `Dispatch`:L777 · `ToPilotObject`:L780 · `ToSExpr`:L830
`ToString`:L832 · `GetCallQuoteKind`:L853 · `HasZeroWhitespaceBeforeParen`:L869 · `ComputeFnMetadataId`:L880
`EvaluateQuotedCall`:L898 · `EvaluateDeepSplice`:L1021 · `BuildAbiCallFrame`:L1087 · `DispatchAbiFrame`:L1153
`IsUnmanagedAbiCompatibleType`:L1166 · `GetAbiScalarSizeAndAlignment`:L1189 · `InferAbiTypeFromValue`:L1207
`WriteScalarToAbiSpan`:L1221 · `ParseSnippetToSuite`:L1266 · `ExtractSymbolicBodyAst`:L1269
`ConvertFunctionBodyToSymbolicAst`:L1275 · `ConvertStmtSequenceToSymbolicAst`:L1296
`ConvertSimpleStmtToSymbolicAst`:L1383 · `ConvertParseTreeToSymbolicAst`:L1403

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

### `Analysis/TypeSystem/PilotBidirectionalInferencer.cs` (4364 lines)
`FromContext`:L39 · `FromToken`:L51 · `ToString`:L60 · `FormatDiagnosticMessage`:L91 · `BuildFunctionType`:L197
`AddField`:L265 · `GetNarrowedTypeAt`:L311 · `GetFlowTypeAtLine`:L314 · `AnalyzeProgram`:L319
`PrescanProgramDeclarations`:L354 · `IsInsideClassOrState`:L417 · `RegisterStructLayout`:L427
`ExtractSuiteFields`:L456 · `RegisterRecordLayout`:L498 · `RegisterClassLayout`:L532 · `RegisterClassMethod`:L577
`FindTransitionTarget`:L630 · `RegisterTypeAliasLayout`:L660 · `RegisterEnumLayout`:L700
`RegisterVariantLayout`:L719 · `HasResourceOrLinearDecorator`:L767 · `RegisterFunctionStub`:L779
`SolveFunctionSccs`:L831 · `CollectCalledSchemes`:L864 · `ComputeTarjanSccs`:L886 · `SolveSingleSccFixedPoint`:L946
`IsTypeVariableReferencedInParameters`:L989 · `InferSchemeOnce`:L1007 · `AnalyzeFunctionBodyFinal`:L1076
`PushFlowScope`:L1169 · `PopFlowScope`:L1177 · `SetFlowNarrowedType`:L1180 · `LookupFlowAwareType`:L1190
`AnalyzeSuiteForReturn`:L1209 · `CheckSuite`:L1228 · `AnalyzeStatement`:L1262 · `AnalyzeSimpleStmt`:L1273
`AnalyzeCompoundStmt`:L1297 · `AnalyzeVarDecl`:L1336 · `AnalyzeVarDecl`:L1347 · `BindDestructuringPattern`:L1410
`AnalyzeGuardStmt`:L1440 · `AnalyzeIfStmt`:L1517 · `AnalyzeMatchStmt`:L1661 · `AnalyzeForStmt`:L1762
`CollectLoopBodyFlowTypes`:L1859 · `AnalyzeFlowStmt`:L1893 · `AnalyzeExprStmt`:L1977
`SubtractAdtOrUnionVariant`:L2189 · `TryExtractPatternNarrowedType`:L2209 · `BindMatchPatternVariables`:L2270
`FindFirstPrimaryPattern`:L2303 · `Check`:L2318 · `CheckAtom`:L2406 · `CheckLambda`:L2467
`ExtractLambdaParameters`:L2560 · `Synthesize`:L2597 · `SynthesizeTest`:L2631 · `SynthesizePipeExpr`:L2658
`SynthesizePipeStage`:L2670 · `SynthesizeSetQueryExpr`:L2686 · `SynthesizeCoalesceExpr`:L2694
`SynthesizeOrTest`:L2744 · `SynthesizeAndTest`:L2752 · `SynthesizeNotTest`:L2760 · `SynthesizeComparison`:L2770
`SynthesizeRangeExpr`:L2791 · `SynthesizeBitorExpr`:L2801 · `SynthesizeBitxorExpr`:L2809
`SynthesizeBitandExpr`:L2817 · `SynthesizeShiftExpr`:L2825 · `SynthesizeArithExpr`:L2833 · `SynthesizeTerm`:L2847
`CombineArithmeticOperandTypes`:L2861 · `ConstrainTypeVariable`:L2889 · `SynthesizeFactor`:L2911
`SynthesizePower`:L2939 · `SynthesizeAtomExpr`:L2950 · `SynthesizeCascadeStep`:L3124
`BindCascadePatternInInference`:L3232 · `InferBuiltinCascadeMemberReturnType`:L3259
`TryRecordRowPolymorphicFieldAccess`:L3277 · `TryInferCollectionHigherOrderMethod`:L3303
`ResolveMemberMethodCall`:L3456 · `ResolveDirectCall`:L3517 · `ResolveSingleSchemeCall`:L3689
`VerifyAndBindRowPolymorphicArgument`:L3761 · `SelectBestFunctionOverload`:L3806 · `SynthesizeAtom`:L3899
`VerifyAndUnifyConstraint`:L4022 · `RecordConstraintFailure`:L4047 · `AddInlayHint`:L4076 · `LookupFieldType`:L4091
`TryLookupFieldAndOffset`:L4098 · `JoinTypes`:L4174 · `InferSimpleSuiteReturnType`:L4213
`TryExtractFailArgumentType`:L4231 · `SuiteAlwaysExits`:L4256 · `SimpleStmtAlwaysExits`:L4279
`FindIfBranchPattern`:L4296 · `HasIfBranchSome`:L4312 · `FindFirstAtomExpr`:L4331 · `IsNullLiteralText`:L4342
`IsSimpleIdentifier`:L4345 · `IsKnownBuiltinTypeName`:L4357 · `TruncateSource`:L4360

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

### `Analysis/TypeSystem/PilotType.cs` (753 lines)
`ListOf`:L104 · `SetOf`:L107 · `MapOf`:L110 · `FutureOf`:L113 · `TupleOf`:L116 · `OptionOf`:L119 · `ResultOf`:L125
`FunctionOf`:L133 · `MatrixOf`:L136 · `WithUnit`:L139 · `WithTypestate`:L142 · `RowRecordOf`:L157 · `UnionOf`:L172
`AddUnique`:L214 · `ResourceOf`:L220 · `NewTypeVariable`:L223 · `WithConstraint`:L226 · `SatisfiesConstraint`:L256
`SubtractType`:L285 · `IsAssignableTo`:L315 · `Unify`:L436 · `OccursIn`:L520 · `Resolve`:L540 · `ResolveCore`:L543
`ToString`:L633 · `FormatConstraint`:L697 · `Equals`:L708 · `Equals`:L734 · `GetHashCode`:L736

### `Cli/Program.cs` (1229 lines)
`Main`:L20 · `PrintVersion`:L264 · `ExecuteInlineScript`:L275 · `ExecuteBuildSubcommand`:L288
`ExecuteCompilationSubcommand`:L404 · `RunScriptFile`:L490 · `ExecuteTestSubcommand`:L624
`ExecuteBenchmarkSubcommand`:L727 · `ExecutePatchSubcommand`:L787 · `ExecuteWatchSubcommand`:L869
`ExecutePackageSubcommand`:L914 · `ExecuteVenvSubcommand`:L1008 · `PrintUsage`:L1083 · `ParseAndReport`:L1120

### `CodeGen/Engine/PilotCodeGeneratorPipeline.cs` (410 lines)
`ShouldRun`:L39 · `Run`:L59 · `Run`:L73 · `ExtractScriptModel`:L165 · `ExtractClassModel`:L212
`ExtractMethodModel`:L287 · `ExtractDecorator`:L326 · `ExtractViaRegexFallback`:L352
`ExecuteInScriptGenerators`:L381

### `Compiler/PilotBytecodeCompiler.cs` (2816 lines)
`IsImmutable`:L41 · `AllocateTempSlot`:L48 · `RegisterBindingMetadata`:L51 · `HasDeclaredBinding`:L71
`EmitStoreBinding`:L82 · `EmitRestoreBinding`:L97 · `EmitLoadBindingOrUndefined`:L110 · `EmitLiteralConstant`:L123
`TryResolveAdtVariant`:L145 · `TryResolveAdtType`:L192 · `TryResolveRecordType`:L207
`IsKnownClassOrStructName`:L222 · `Compile`:L247 · `CompileSuite`:L259 · `PrescanProgramTypeDeclarations`:L303
`RegisterAdtCompileTime`:L343 · `RegisterRecordCompileTime`:L352 · `TryBuildTypeAliasAdtDescriptor`:L357
`BuildEnumDescriptor`:L445 · `BuildRecordDescriptor`:L473 · `ExtractGenericParamList`:L491
`ExtractSerializedFields`:L497 · `VisitProgram`:L534 · `VisitStmt`:L548 · `VisitSimple_stmt`:L557
`VisitSmall_stmt`:L572 · `VisitCompound_stmt`:L587 · `VisitType_alias_stmt`:L613 · `VisitEnum_def`:L634
`VisitRecord_def`:L652 · `VisitMatch_stmt`:L670 · `VisitGuard_stmt`:L676 · `VisitSuite`:L682
`IsSliceBufferTypeAnnotation`:L704 · `IsTopLevelForExpression`:L716 · `VisitLet_stmt`:L725 · `VisitVar_stmt`:L749
`VisitConst_stmt`:L773 · `ExtractBindingIdentifiers`:L797 · `CompileVariableDeclaration`:L843
`VisitExpr_stmt`:L869 · `AssignToTarget`:L923 · `CompileAugmentedAssign`:L989 · `EmitAugassignOp`:L1117
`VisitFlow_stmt`:L1135 · `VisitIf_stmt`:L1181 · `VisitWhile_stmt`:L1239 · `VisitFor_stmt`:L1288
`VisitFunc_def`:L1361 · `VisitTest`:L1410 · `VisitPipe_expr`:L1437 · `TryCompileFluentPipeStage`:L1458
`VisitSet_query_expr`:L1528 · `VisitCoalesce_expr`:L1537 · `CompileCoalesceExprPrefix`:L1543
`CompileCatchBlockAsExpression`:L1606 · `VisitOr_test`:L1624 · `VisitAnd_test`:L1641 · `VisitNot_test`:L1658
`VisitComparison`:L1677 · `VisitRange_expr`:L1715 · `VisitBitor_expr`:L1732 · `VisitBitxor_expr`:L1746
`VisitBitand_expr`:L1760 · `VisitShift_expr`:L1774 · `VisitArith_expr`:L1789 · `VisitTerm`:L1804
`VisitFactor`:L1826 · `VisitPower`:L1856 · `VisitAtom_expr`:L1869 · `RequiresAtomExprAstFallback`:L1978
`RequiresCascadeAstFallback`:L2034 · `PreallocateCascadeFallbackBindings`:L2101
`PreallocateSingleCascadeBindings`:L2112 · `CompileCascadeTrailer`:L2149 · `EmitCascadeCaptureStore`:L2246
`HasNamedArguments`:L2310 · `TryResolveKnownFieldOrderForCall`:L2321 · `OrderCallArgumentExpressions`:L2361
`VisitAtom`:L2409 · `VisitLiteral`:L2572 · `VisitCollection_expr`:L2703 · `IsSimpleIdentifier`:L2736
`GetAtomExpr`:L2752 · `CompileAtomExprPrefix`:L2760 · `UnescapeStringLiteral`:L2785 · `StripQuotes`:L2801

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

### `Native/CAbi/PilotCAbiEmitter.cs` (554 lines)
`EmitHeader`:L24 · `EmitHeader`:L43 · `SanitizeCIdentifier`:L189 · `Extract`:L225 · `PrescanTypes`:L256
`ExtractGenerics`:L333 · `ExtractAdtFields`:L336 · `VisitFunc_def`:L359 · `VisitStruct_def`:L385
`VisitClass_def`:L402 · `MapType`:L443

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

### `Pilot.LanguageServer/Lsp/PilotLspServer.cs` (859 lines)
`RunAsync`:L100 · `ProcessMessage`:L121 · `UpdateDocumentAndBuildDiagnostics`:L610 · `TryGetAnalysis`:L643
`ConvertToDocumentSymbol`:L660 · `GetCompletionItemKind`:L679 · `CreateRange`:L694 · `BuildResponse`:L700
`ExtractWordAt`:L711 · `SendRawJson`:L758 · `ReadMessageAsync`:L776

### `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (5217 lines)
`IsBuiltinType`:L51 · `Analyze`:L347 · `SynchronizeInferredTypesFromAnalysis`:L424 · `SynchronizeScopeSymbols`:L550
`FindFunctionSymbol`:L591 · `FindFunctionScope`:L603 · `FormatInferredTypeForSymbol`:L615
`CheckUnusedVariables`:L636 · `PreRegisterTopLevelDeclarations`:L673 · `RegisterFunctionStub`:L763
`RegisterTypeStub`:L788 · `DeclareBindingPattern`:L812 · `VisitLet_stmt`:L871 · `VisitVar_stmt`:L886
`VisitConst_stmt`:L921 · `VisitRef_stmt`:L936 · `VisitDynamic_stmt`:L951 · `VisitAnnassign_stmt`:L959
`VisitType_alias_stmt`:L990 · `VisitPlainTypeAlias`:L999 · `VisitExpr_stmt`:L1012
`CheckImmutableMemberAssignment`:L1122 · `StripGenericSuffixForLookup`:L1177 · `FindFirstAtomExpr`:L1184
`VisitGuard_stmt`:L1195 · `VisitDefer_stmt`:L1262 · `VisitFunc_def`:L1268 · `VisitContract_clause`:L1417
`VisitPattern_assign_stmt`:L1430 · `VisitClass_def`:L1439 · `VisitStruct_def`:L1471 · `VisitInterface_def`:L1500
`VisitEnum_def`:L1516 · `VisitActor_def`:L1558 · `VisitModel_def`:L1574 · `VisitProperty_def`:L1591
`VisitOperator_def`:L1666 · `VisitExtension_def`:L1703 · `VisitAdapter_def`:L1724 · `VisitUnit_stmt`:L1754
`VisitDsl_stmt`:L1770 · `VisitMacro_def`:L1786 · `VisitQuote_stmt`:L1817 · `VisitQuote_expr`:L1827
`VisitQuasiquote_expr`:L1836 · `VisitSplice_expr`:L1848 · `VisitForeign_import_stmt`:L1860
`VisitExtern_def`:L1955 · `ProcessExternMember`:L2018 · `ProcessExternFunction`:L2034 · `ProcessExternType`:L2110
`VisitInline_foreign_block`:L2164 · `VisitDatabase_stmt`:L2182 · `VisitTransaction_stmt`:L2193
`VisitSavepoint_stmt`:L2206 · `VisitSavepoint_block`:L2217 · `VisitRollback_stmt`:L2229 · `VisitNoalloc_stmt`:L2251
`VisitWhile_stmt`:L2261 · `VisitFor_stmt`:L2295 · `VisitIf_stmt`:L2398 · `VisitFlow_stmt`:L2418
`VisitPass_stmt`:L2570 · `VisitAssert_stmt`:L2577 · `VisitComparison`:L2584 · `VisitRange_expr`:L2604
`VisitTest`:L2613 · `VisitComp_op`:L2630 · `VisitSelect_stmt`:L2638 · `VisitSelect_arm`:L2657
`VisitQuery_expr`:L2681 · `VisitSql_query_expr`:L2730 · `IsInsideSyncFunction`:L2758 · `VisitUnary_op`:L2775
`VisitPrimary_type`:L2807 · `VisitAtom_expr`:L2851 · `VisitTerm`:L3031 · `VisitLabeled_stmt`:L3057
`VisitBlock`:L3067 · `VisitLambda_expr`:L3076 · `VisitMacro_call`:L3160 · `VisitSymbol_literal`:L3172
`VisitCascade_trailer`:L3188 · `VisitNameof_expr`:L3295 · `VisitAtom`:L3314 · `InferLiteralType`:L3506
`ValidateAndResolveType`:L3529 · `IsCompatibleType`:L3557 · `IsStrictPrimitive`:L3664 · `NormalizePrimitive`:L3670
`TryExtractUnit`:L3678 · `ExpandUnitExpression`:L3691 · `SuiteGuaranteesExit`:L3702 · `SimpleStmtExits`:L3715
`VisitSimple_stmt`:L3722 · `VisitSuite`:L3766 · `StmtGuaranteesExit`:L3812 · `VisitArith_expr`:L3828
`VisitExcept_clause`:L3868 · `VisitMatch_stmt`:L3889 · `VisitMatch_arm`:L3912 · `VisitMatchArmWithScrutinee`:L3917
`BindPatternIdentifiers`:L3959 · `VisitRecord_def`:L3965 · `RegisterAdtSumTypeAlias`:L4023
`RegisterAdtVariantSymbol`:L4064 · `FinishGuardSuite`:L4102 · `VisitIfBranchesWithPatterns`:L4120
`BindPatternVarSingle`:L4229 · `BindPatternNode`:L4252 · `BindPatternNodeLegacyUnused`:L4420
`RegisterGenericParams`:L4455 · `DeclareVariableSymbol`:L4467 · `DeclareVariableSymbolFromToken`:L4482
`CreateSymbol`:L4516 · `CreateSymbolFromToken`:L4538 · `RegisterSymbol`:L4588 · `AddSymbolReferenceDirect`:L4615
`AddSemanticTokenDirect`:L4627 · `ProcessFStringNode`:L4648 · `AddSymbolReference`:L4786 · `AddSemanticToken`:L4799
`AddDiagnostic`:L4812 · `AddDiagnosticDirect`:L4835 · `GetSemanticTokenTypeForSymbol`:L4857
`GetTokenCoords`:L4874 · `ExtractDictTypes`:L4883 · `ExtractElementType`:L4911 · `IsSubtypeOf`:L4954
`FindCommonBaseType`:L4967 · `HomogenizeTypes`:L5001 · `TryExtractSimpleIdentifier`:L5074
`FindFirstIdentifier`:L5083 · `Truncate`:L5094 · `EnterScope`:L5097 · `Dispose`:L5143 · `SyntaxError`:L5151

### `Pilot.LanguageServer/Semantics/PilotSymbol.cs` (393 lines)
`GetLspDocumentSymbolKind`:L189 · `FormatHoverMarkdown`:L212 · `Resolve`:L286 · `GetAllVisibleSymbols`:L296
`FindSymbolAtPosition`:L321 · `FindScopeAtLine`:L332 · `FindDeepestScope`:L337 · `EncodeSemanticTokens`:L347

### `Preprocessing/PilotPreprocessor.cs` (983 lines)
`ToString`:L24 · `IsWarningSuppressed`:L69 · `Preprocess`:L103 · `ProcessDirective`:L219 · `ProcessPragma`:L447
`EvaluateCondition`:L490 · `SplitLines`:L518 · `IsValidSymbolName`:L556 · `StripQuotes`:L571
`TokenizeCondition`:L588 · `Parse`:L661 · `ParseOr`:L669 · `ParseAnd`:L681 · `ParseEquality`:L693
`ParseUnary`:L705 · `ParsePrimary`:L715 · `GetIndent`:L767 · `FindTrailingCommentIndex`:L779
`IsBlockStartLine`:L801 · `EndsWithBlockColon`:L831 · `DesugarColonBlocks`:L877

### `Runtime/Builtins/PilotBuiltinRegistry.cs` (1155 lines)
`Register`:L25 · `Register`:L30 · `TryGet`:L38 · `GetAll`:L41 · `RegisterDefaults`:L43
`GetOperatorFunction`:L1133

### `Runtime/Bytecode/PilotBytecodeChunk.cs` (586 lines)
`Emit`:L89 · `EmitJump`:L97 · `PatchJump`:L107 · `AddJumpTable`:L121 · `AddTypeDescriptor`:L130
`AddConstant`:L139 · `AddIdentifier`:L155 · `AddNestedChunk`:L171 · `AddAstFallback`:L180
`GetOrAllocateLocalSlot`:L189 · `TryGetLocalSlot`:L203 · `Write`:L210 · `Read`:L304 · `ReadV2Extensions`:L392
`WriteValue`:L453 · `ReadValue`:L522 · `ReadKeywordVariant`:L545 · `ReadList`:L554 · `ReadTuple`:L565
`ReadRange`:L576

### `Runtime/Bytecode/PilVirtualMachine.cs` (1530 lines)
`Execute`:L28 · `DeclareTypeDescriptorInContext`:L1158 · `RefreshLayouts`:L1211 · `CheckRuntimeTypeMatch`:L1223
`MatchesClassHierarchy`:L1326 · `EnsureStackCapacity`:L1347 · `InvokeCallable`:L1356 · `InvokeMethod`:L1395
`CreateIterator`:L1412 · `EnumerateSliceBuffer`:L1455 · `EnumerateRange`:L1463 · `EnumerateString`:L1477
`EnumerateObjects`:L1485 · `EnumerateSingle`:L1493 · `CreateParameters`:L1498

### `Runtime/Effects/PilotEffectDryRunEngine.cs` (767 lines)
`ToPilotObject`:L61 · `ToString`:L88 · `Inspect`:L145 · `Commit`:L162 · `Rollback`:L178 · `ToPilotObject`:L187
`NotifyObjectCreated`:L248 · `NotifyScopeCreated`:L256 · `EvaluateEffectDryRun`:L267 · `ExecuteCommitActions`:L326
`TryReadObjectField`:L345 · `TryRecordObjectFieldWrite`:L361 · `TryReadScopeVariable`:L396
`TryRecordScopeVariableWrite`:L412 · `GetReadableList`:L450 · `TryRecordListIndexWrite`:L464
`TryRecordListAppend`:L504 · `TryReadMapEntry`:L531 · `TryRecordMapWrite`:L546 · `TryReadSliceBufferElement`:L573
`TryRecordSliceBufferWrite`:L589 · `TryReadNativeArrayElement`:L612 · `TryRecordNativeArrayWrite`:L628
`TryRecordDatabaseExec`:L655 · `TryReadFileContent`:L679 · `TryRecordFileWrite`:L696 · `TryRecordFileDelete`:L735

### `Runtime/Execution/PilotEngine.cs` (650 lines)
`Execute`:L21 · `Execute`:L27 · `ExecuteFile`:L153 · `EvaluateExpression`:L168 · `InvokeFunction`:L186
`CreateGenerator`:L341 · `CreateGenerator`:L381 · `RunnerBody`:L386 · `Next`:L441 · `Close`:L471
`TranspileTo`:L486 · `TranspileFrom`:L492 · `Compile`:L498 · `CompileFile`:L508 · `CreateCallScope`:L518
`SnapshotOldCalls`:L581 · `Write`:L627 · `Write`:L633 · `WriteLine`:L639 · `Flush`:L645

### `Runtime/Execution/PilotInterpreterVisitor.cs` (5739 lines)
`VisitProgram`:L84 · `VisitStmt`:L129 · `VisitSimple_stmt`:L143 · `VisitSuite`:L160 · `VisitLet_stmt`:L184
`VisitVar_stmt`:L192 · `VisitConst_stmt`:L200 · `EvaluateDeclarationInitializer`:L208 · `TryUnwrapForStmt`:L283
`DeclareBinding`:L293 · `DeclareRecordPattern`:L321 · `ProcessRecordRestPattern`:L349
`ProcessRecordFieldPattern`:L381 · `DeclareListPattern`:L443 · `DeclareTuplePattern`:L516
`VisitPattern_assign_stmt`:L596 · `VisitExpr_stmt`:L614 · `TryUnwrapAtomExpr`:L691 · `AnalyzeTarget`:L731
`EvaluateAtomExprPrefix`:L808 · `AssignTarget`:L895 · `VisitFunc_def`:L1030 · `ContainsYield`:L1129
`ContainsYieldCore`:L1135 · `VisitClass_def`:L1152 · `VisitState_def`:L1317 · `VisitMacro_def`:L1370
`VisitExtern_def`:L1405 · `VisitType_alias_stmt`:L1459 · `VisitEnum_def`:L1522 · `VisitRecord_def`:L1541
`VisitStruct_def`:L1580 · `DefaultValueForType`:L1640 · `RegisterAdtDefinition`:L1649
`RefreshAllAdtAndRecordLayouts`:L1672 · `IsKnownTypeOrBinding`:L1684 · `ExtractGenericParamNames`:L1700
`ExtractAdtFields`:L1706 · `VisitExtension_def`:L1735 · `EnsureSelfParameter`:L1848
`SanitizeExtensionTargetName`:L1861 · `VisitAdapter_def`:L1871 · `VisitSteal_stmt`:L1918
`VisitDelegate_stmt`:L1958 · `VisitContract_forward_stmt`:L1979 · `VisitIf_stmt`:L2004
`HasBranchSomeKeyword`:L2084 · `FindBranchPattern`:L2104 · `FindBranchPatternGuard`:L2123
`IsBareOptionalBindingPattern`:L2142 · `VisitWhile_stmt`:L2167 · `VisitLabeled_stmt`:L2293 · `VisitBlock`:L2324
`VisitFor_stmt`:L2347 · `TryGetSimpleForTargetName`:L2794 · `VisitWith_stmt`:L2812 · `AssignForTarget`:L2893
`VisitMatch_stmt`:L2919 · `EvaluateMatchSuite`:L2948 · `VisitGuard_stmt`:L2975 · `EvaluateGuardRhs`:L3074
`VisitDefer_stmt`:L3121 · `IsKnownExceptionTypeName`:L3178 · `VisitTry_stmt`:L3188 · `IsExceptionMatching`:L3305
`VisitAssert_stmt`:L3329 · `VisitFlow_stmt`:L3343 · `VisitTestlist`:L3472 · `VisitTest`:L3489
`VisitLambda_expr`:L3514 · `VisitPipe_expr`:L3590 · `VisitSet_query_expr`:L3607 · `VisitCoalesce_expr`:L3616
`VisitCoalesceExprPrefix`:L3621 · `EvaluateCatchClause`:L3658 · `VisitOr_test`:L3695 · `VisitAnd_test`:L3709
`VisitNot_test`:L3723 · `VisitComparison`:L3736 · `VisitRange_expr`:L3779 · `VisitBitor_expr`:L3794
`VisitBitxor_expr`:L3808 · `VisitBitand_expr`:L3822 · `VisitShift_expr`:L3836 · `VisitArith_expr`:L3853
`VisitTerm`:L3870 · `EvaluateProjection`:L3922 · `VisitFactor`:L3982 · `VisitUnary_op`:L3992 · `VisitPower`:L4031
`VisitAtom_expr`:L4046 · `CreateFluentMemberFunction`:L4216 · `TryInvokeDirectMemberCall`:L4258
`EvaluateCascade`:L4389 · `ApplyHoistedCascades`:L4534 · `BindCascadeCapturedValue`:L4559 · `DispatchCall`:L4582
`InvokeCallableSingle`:L4726 · `ExtractCollectionItems`:L4846 · `EvaluateCallArguments`:L4901
`EvaluateArglist`:L4907 · `BindArgumentsToFunction`:L4912 · `IsCollectionOrTupleParameterType`:L4992
`VisitAtom`:L5022 · `VisitSymbol_literal`:L5054 · `VisitMacro_call`:L5075 · `IsBuiltinSpecialFormName`:L5092
`ResolveIdentifier`:L5097 · `VisitLiteral`:L5117 · `ParseDurationLiteralSeconds`:L5226
`ParseFrequencyLiteralHz`:L5244 · `ParseMemoryLiteralBytes`:L5258 · `IsSimpleIdentifier`:L5274
`ParseFStringSegments`:L5287 · `EvaluateFString`:L5333 · `UnescapeString`:L5377 · `VisitCollection_expr`:L5390
`VisitDictorsetmaker`:L5421 · `VisitTestlist_comp`:L5504 · `VisitInline_foreign_block`:L5520
`GetBlockBodyText`:L5645 · `VisitInsert_stmt`:L5677 · `VisitUpdate_stmt`:L5704 · `VisitDelete_stmt`:L5727

### `Runtime/Execution/PilotPatternMatcher.cs` (986 lines)
`Match`:L25 · `MatchPatternInternal`:L46 · `MatchOrPattern`:L57 · `MatchAndPattern`:L86 · `MatchNotPattern`:L103
`MatchPrimaryPattern`:L134 · `MatchRangePattern`:L249 · `EvaluateRangeBound`:L295 · `ParseSignedNumber`:L315
`ParseUnsignedIntToken`:L326 · `MatchKeywordPattern`:L335 · `MatchTypeOrVarPattern`:L396 · `MatchAdtFields`:L732
`MatchRecordFields`:L780 · `MatchPropertyPattern`:L819 · `MatchesObjectType`:L863 · `MatchesPrimitiveType`:L891
`MatchCollectionSlice`:L914

### `Runtime/HotReload/PilotHotReloader.cs` (732 lines)
`CreateNonExistent`:L50 · `Capture`:L54 · `Restore`:L107 · `ToPilotMap`:L190 · `ToPilotMap`:L218
`ApplyCodeOrFile`:L262 · `ApplySourcePatch`:L287 · `ApplyCompiledPatch`:L311 · `ApplyPatchFile`:L459
`Rollback`:L522 · `WatchFile`:L573 · `UnwatchAll`:L634 · `Dispose`:L645 · `IsCompiledBinary`:L650
`RestoreScopeChain`:L663 · `ExtractTopLevelDeclaredSymbols`:L689 · `ComputeSha256Hex`:L727

### `Runtime/Interop/PolyglotRegistry.cs` (1572 lines)
`Register`:L40 · `TryGet`:L46 · `NormalizeLanguage`:L52 · `Get`:L64 · `ExecuteBlock`:L88 · `Invoke`:L119
`Import`:L137 · `EvaluatePythonLiteralOrMarshal`:L142 · `ExecuteBlock`:L170 · `Invoke`:L198 · `Import`:L213
`EvaluateCSharpLiteralOrMarshal`:L218 · `ExecuteBlock`:L246 · `Invoke`:L269 · `Import`:L274 · `ExecuteBlock`:L293
`Invoke`:L331 · `Import`:L334 · `ExecuteBlock`:L351 · `Invoke`:L383 · `Import`:L386 · `ExecuteBlock`:L403
`Invoke`:L533 · `Import`:L536 · `ExecuteBlock`:L553 · `Invoke`:L582 · `Import`:L585 · `ExecuteBlock`:L602
`Invoke`:L630 · `Import`:L633 · `ExecuteBlock`:L650 · `Invoke`:L680 · `Import`:L683 · `ExecuteBlock`:L700
`ExecuteSqlQuery`:L705 · `Invoke`:L754 · `Import`:L757 · `ExecuteBlock`:L774 · `Invoke`:L777 · `Import`:L780
`LocateExecutable`:L793 · `CollectReferencedBuffers`:L819 · `ContainsIdentifierToken`:L854
`TryExecutePythonSharedMemory`:L872 · `TryExecuteNodeSharedMemory`:L995 · `ParseSubprocessReturn`:L1089
`TryExecuteInProcessBufferScript`:L1125 · `SplitForeignStatements`:L1156 · `ExecuteForeignStatementList`:L1190
`TryMatchBufferAliasRhs`:L1325 · `EvaluateNumericExpr`:L1345 · `EvaluateNumericExpr`:L1351
`IsBalancedOuterParens`:L1418 · `FindTopLevelBinaryOp`:L1433 · `ResolveAsmScalarOperand`:L1455
`ResolveAsmFloatOperand`:L1501 · `TryResolveAsmMemoryAddress`:L1519

### `Runtime/Interop/PolyglotTypeBridge.cs` (490 lines)
`RegisterCustomMapper`:L24 · `ToPilotValue`:L30 · `TryMapPrimitiveToPilotValue`:L66
`TryMapCollectionToPilotValue`:L179 · `ToForeignValue`:L235 · `MapKindToForeignValue`:L260
`TryCoerceToForeignType`:L285 · `ConvertPocoToPilotObject`:L328 · `ConvertListToHost`:L374
`ConvertMapToHost`:L384 · `InferForeignReturnPilotType`:L400 · `InferForeignClrPilotType`:L431
`InferSqlColumnPilotType`:L455 · `MapClrBaseType`:L478

### `Runtime/NativeMemory/PilotSharedBuffer.cs` (750 lines)
`AllocateUnmanaged`:L100 · `CreateOrOpenShared`:L121 · `WrapExternalPointer`:L140 · `MapBackingFile`:L156
`EnsureSharedMapping`:L202 · `OpenIndependentView`:L248 · `SliceView`:L260 · `ResetLength`:L276
`ReserveInitialCapacity`:L286 · `EnsureCapacityForYield`:L305 · `YieldElement`:L337 · `GetElement`:L348
`SetElement`:L368 · `WriteRawUnchecked`:L384 · `AsSpan`:L410 · `CapacitySpan`:L423 · `AsTypedSliceBuffer`:L436
`Sum`:L444 · `ScaleInPlace`:L495 · `Fill`:L537 · `ToPilotValueList`:L548 · `Flush`:L557 · `Equals`:L562
`Equals`:L575 · `GetHashCode`:L577 · `ToString`:L579 · `GetStride`:L593 · `GetCanonicalElementName`:L603
`GetPythonStructFormat`:L613 · `ParseElementKind`:L623 · `ResolveBackingFilePath`:L655 · `EnsureNotDisposed`:L683
`ReleaseCurrentMapping`:L689 · `Dispose`:L717 · `Dispose`:L740

### `Runtime/Packaging/PilotPackageManager.cs` (376 lines)
`RegisterAdapter`:L34 · `TryGetAdapter`:L42 · `Init`:L50 · `Add`:L85 · `Remove`:L123 · `Restore`:L159 · `List`:L246
`Pack`:L296 · `ParsePackageSpec`:L338 · `ComputeHash`:L361 · `GetEffectiveModulesDirectory`:L367

### `Runtime/Reactive/PilotSignalGraph.cs` (812 lines)
`Read`:L40 · `Subscribe`:L42 · `NotifyWatchers`:L48 · `ToPilotObject`:L61 · `ResolveDynamicMember`:L114
`SynchronizeObjectFields`:L128 · `Read`:L176 · `Set`:L191 · `Read`:L263 · `Recompute`:L265 · `Compare`:L290
`Equals`:L311 · `Equals`:L316 · `GetHashCode`:L318 · `Equals`:L333 · `Equals`:L338 · `GetHashCode`:L340
`CreateSourceSignal`:L357 · `GetOrCreatePropertySignal`:L360 · `GetOrCreateVariableSignal`:L375
`NotifyObjectPropertyChanged`:L390 · `NotifyScopeVariableChanged`:L402 · `Batch`:L418 · `Batch`:L435
`PropagateFrom`:L452 · `FlushDirtyQueue`:L470 · `TryExtractSignalNode`:L498 · `CreateReactiveCallSignal`:L515
`EvaluateReactiveArgumentExpr`:L598 · `IsSimpleIdentifier`:L613 · `ExtractObservableDependenciesFromTree`:L624
`EvaluateAtomAndTrailersPrefix`:L728 · `HasStaticObservableInput`:L758 · `TreeContainsStaticObservable`:L775

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

### `Runtime/Values/PilotMethodGroup.cs` (635 lines)
`AddOverload`:L54 · `CloneSnapshot`:L73 · `RestoreFromSnapshot`:L79 · `Union`:L92 · `BindTo`:L107 · `Invoke`:L116
`ExtractCollectionElements`:L250 · `ExecuteOverload`:L258 · `TryMatchOverload`:L276 · `CanPartiallyMatch`:L362
`IsTypeCompatible`:L399 · `IsTypeCompatible`:L404 · `IsDirectlyCompatible`:L425
`ComputeOverloadSpecificityScore`:L518 · `MatchesReifiedType`:L559 · `MatchesSingleTypeFunction`:L571
`GetOverloadValues`:L584 · `BuildSignature`:L615 · `ToString`:L633

### `Runtime/Values/PilotObject.cs` (665 lines)
`Union`:L31 · `Subtract`:L59 · `Subtract`:L93 · `Intersect`:L107 · `CloneWithName`:L134 · `MergeMethodValue`:L151
`CloneSnapshot`:L209 · `RestoreFromSnapshot`:L227 · `StealFrom`:L295 · `DelegateContract`:L330 · `Instantiate`:L345
`PopulateDefaultFields`:L373 · `ToString`:L387 · `GetMember`:L416 · `SetMember`:L568 · `BindMethodValue`:L578
`BindMethod`:L611 · `Clone`:L613 · `Equals`:L623 · `Equals`:L637 · `GetHashCode`:L639 · `ToString`:L651

### `Runtime/Values/PilotValue.cs` (2330 lines)
`CreateTable`:L38 · `FromBool`:L63 · `FromInt`:L64 · `FromUInt`:L65 · `FromFloat`:L66 · `FromDecimal`:L67
`FromChar`:L68 · `FromString`:L69 · `FromKeyword`:L70 · `FromKeyword`:L71 · `FromList`:L72 · `FromTuple`:L73
`FromMap`:L74 · `FromSet`:L75 · `FromRange`:L76 · `FromMatrix`:L77 · `FromFunction`:L78 · `FromObject`:L79
`FromGenerator`:L80 · `FromLazy`:L81 · `FromFuture`:L82 · `FromForeign`:L83 · `FromMethodGroup`:L84
`FromNativeArray`:L85 · `FromNativeMap`:L86 · `FromSliceBuffer`:L87 · `FromAdtType`:L88 · `FromAdtUnit`:L89
`FromAdtBoxed`:L90 · `FromRecordUnboxedEmpty`:L91 · `FromRecordUnboxedPairF64`:L92 · `FromRecordBoxed`:L94
`FromKeywordVariant`:L95 · `FromSome`:L103 · `UnwrapOption`:L120 · `FromFailure`:L140 · `UnwrapFailure`:L160
`TryCreateUnboxedAdtSingle`:L193 · `PilotValue`:L216 · `PilotValue`:L217 · `PilotValue`:L218 · `PilotValue`:L219
`PilotValue`:L220 · `PilotValue`:L221 · `PilotValue`:L222 · `PilotValue`:L223 · `PilotValue`:L224 · `AsInt`:L253
`AsFloat`:L265 · `AsString`:L277 · `AsDecimal`:L278 · `AsChar`:L279 · `AsKeyword`:L280 · `AsList`:L282
`AsTuple`:L283 · `AsMap`:L284 · `AsSet`:L285 · `AsRange`:L286 · `AsMatrix`:L287 · `AsFunction`:L288
`AsObject`:L289 · `AsGenerator`:L290 · `AsLazy`:L291 · `AsFuture`:L292 · `AsMethodGroup`:L293
`AsNativeArray`:L294 · `AsNativeMap`:L295 · `AsSliceBuffer`:L296 · `AsForeign`:L297 · `AsAdtType`:L305
`AsKeywordVariant`:L306 · `GetAdtVariantDefinition`:L308 · `GetAdtTag`:L316 · `GetAdtFieldCount`:L318
`GetAdtField`:L320 · `GetRecordDefinition`:L343 · `GetRecordFieldCount`:L350 · `GetRecordField`:L352
`CreateClassConstructor`:L372 · `GetMember`:L730 · `CreateParseError`:L1357 · `ComputeCollectionSum`:L1366
`InvokeSingleCallback`:L1389 · `MapCollection`:L1406 · `FilterCollection`:L1416
`TryInvokePrimitiveIntrinsic`:L1434 · `EnumerateCollection`:L1949 · `CreateAppendFunction`:L1957
`ResolveExtensionOrRoot`:L2004 · `ResolveRootMember`:L2042 · `CloneValue`:L2095 · `Equals`:L2111
`EqualsAdt`:L2171 · `EqualsRecord`:L2186 · `ComputeAdtHashCode`:L2201 · `ComputeRecordHashCode`:L2211
`FormatAdt`:L2220 · `FormatRecord`:L2230 · `EqualsList`:L2244 · `EqualsTuple`:L2251 · `EqualsMap`:L2258
`Equals`:L2268 · `GetHashCode`:L2270 · `ToString`:L2285 · `FormatNestedSome`:L2322

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

### `vscode-pilot/src/extension.ts` (711 lines)
`pickNewestExistingFile`:L18 · `resolvePilotExecutionPrefix`:L40 · `activate`:L332 · `addOrUpdateLaunchJson`:L633
`deactivate`:L711

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
- `Analysis/Autodiff/PilotSymbolicDifferentiator.cs` (759 L) — struct `Dual_ComputeImpulse`:L20, record `PilotCseBinding`:L48, class `PilotDualValue`:L59, class `PilotSymbolicDifferentiator`:L181
- `Analysis/CallQuoting/PilotCallQuotingEngine.cs` (1661 L) — enum `CallQuoteKind`:L22, class `PilotCallExpr`:L38, enum `PilotAstNodeKind`:L125, class `PilotAstExpr`:L142, struct `CallFrame_ComputeImpulse`:L707, class `PilotAbiCallFrame`:L727, class `PilotCallQuotingEngine`:L845
- `Analysis/ContractVerification/PilotContractVerifier.cs` (227 L) — class `PilotContractVerifier`:L14, class `FunctionContractInfo`:L21
- `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs` (344 L) — class `UnitDimensionSignature`:L13, class `PilotDimensionChecker`:L101
- `Analysis/Loops/PilotForLoopAnalyzer.cs` (473 L) — enum `LoopScheduleMode`:L11, enum `LoopReturnMode`:L26, class `PilotForLoopInfo`:L41, class `PilotForLoopAnalyzer`:L58, class `ScopedBodyScanState`:L453
- `Analysis/Ownership/PilotBorrowChecker.cs` (503 L) — enum `ResourceState`:L13, enum `ResourceAffinity`:L25, class `ResourceBinding`:L44, class `PilotBorrowChecker`:L68
- `Analysis/PatternMatching/PilotExhaustivenessChecker.cs` (2046 L) — class `PilotExhaustivenessChecker`:L21, record `MatchPat`:L536, record `WildPat`:L537, record `OrPat`:L541, record `CtorPat`:L542, enum `CtorFamily`:L544, record `CtorDesc`:L563
- `Analysis/PilotAnalysisPipeline.cs` (156 L) — class `PilotStaticAnalysisResult`:L17, class `PilotAnalysisPipeline`:L54, class `NullAssertionSyntaxListener`:L132
- `Analysis/TypeSystem/PilotBidirectionalInferencer.cs` (4364 L) — class `PilotSourceSpan`:L14, class `PilotTypeConstraint`:L66, enum `PilotInlayHintKind`:L99, class `PilotInlayHint`:L109, class `PilotMonomorphizedCallSite`:L142, class `PilotFunctionScheme`:L178, class `PilotNominalTypeLayout`:L245, class `PilotBidirectionalInferencer`:L278
- `Analysis/TypeSystem/PilotErrorFlowAnalyzer.cs` (1240 L) — class `PilotErrorFlowAnalyzer`:L19, class `FallibleFunctionInfo`:L27, class `ErrorScope`:L45
- `Analysis/TypeSystem/PilotNullSafetyAnalyzer.cs` (1481 L) — class `PilotNullSafetyAnalyzer`:L16, class `FunctionSignatureInfo`:L18, class `TypeMemberInfo`:L26, class `NullSafetyScope`:L33
- `Analysis/TypeSystem/PilotType.cs` (753 L) — enum `PilotTypeKind`:L10, enum `PilotTypeClassConstraint`:L32, class `PilotType`:L47
- `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (336 L) — class `PilotAnalysisDiagnostic`:L13, class `PilotTypeInferenceEngine`:L40

### antlr
- `antlr/concurrency/ConcurrencyLexer.g4` (23 L)
- `antlr/concurrency/ConcurrencyParser.g4` (51 L)
- `antlr/core/CoreLexer.g4` (222 L)
- `antlr/core/CoreParser.g4` (730 L)
- `antlr/csharp/CSharpLexer.g4` (13 L)
- `antlr/csharp/CSharpParser.g4` (54 L)
- `antlr/database/DatabaseLexer.g4` (68 L)
- `antlr/database/DatabaseParser.g4` (163 L)
- `antlr/Pilot.g4` (1712 L)
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
- `Compiler/PilotBytecodeCompiler.cs` (2816 L) — class `PilotBytecodeCompiler`:L17, class `LoopContext`:L225
- `Compiler/PilotCompilationException.cs` (19 L) — class `PilotCompilationException`:L8
- `Compiler/PilotCompilationResult.cs` (42 L) — class `PilotCompilationResult`:L9
- `Compiler/PilotCompiledProgram.cs` (607 L) — class `PilotCompiledProgram`:L20
- `Compiler/PilotCompiler.cs` (197 L) — class `PilotCompiler`:L17
- `Compiler/PilotCompilerOptions.cs` (110 L) — enum `CompilationTarget`:L9, class `PilotCompilerOptions`:L31
- `Compiler/PilotMatchBytecodeLowerer.cs` (2247 L) — class `PilotMatchBytecodeLowerer`:L18, class `StagedBinding`:L29, enum `ArmBranchKind`:L43, class `AnalyzedBranch`:L52
- `Compiler/PilotParserFast.cs` (186 L) — class `PilotParser`:L13

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
- `examples/showcase/19_expression_ergonomics_and_extensions.pilot` (51 L)
- `examples/showcase/20_dynamic_for_expressions_and_c_abi.pilot` (94 L)
- `examples/showcase/21_advanced_type_inference.pilot` (33 L)
- `examples/showcase/22_homoiconic_call_quoting.pilot` (40 L)
- `examples/showcase/23_semantic_call_projections.pilot` (38 L)
- `examples/showcase/run_all.ps1` (14 L)

### Native
- `Native/Assembly/PilotAssemblyEmitter.cs` (213 L) — class `PilotAssemblyEmitter`:L13
- `Native/CAbi/PilotCAbiEmitter.cs` (554 L) — class `PilotCAbiEmitter`:L19, class `StructDescriptor`:L195, class `AdtDescriptor`:L201, class `FunctionDescriptor`:L207, class `CAbiExtractionVisitor`:L214
- `Native/Llvm/PilotLlvmEmitter.cs` (1869 L) — class `PilotLlvmEmitter`:L21
- `Native/PilotNativeCompiler.cs` (373 L) — enum `NativeCompilationTarget`:L16, class `PilotNativeOptions`:L31, class `PilotNativeResult`:L46, class `PilotNativeCompiler`:L90
- `Native/Runtime/pilot_runtime.h` (182 L)
- `Native/Wasm/PilotWasmEmitter.cs` (878 L) — class `PilotWasmEmitter`:L18

### Pilot.LanguageServer
- `Pilot.LanguageServer/Lsp/PilotLspServer.cs` (859 L) — class `PilotLspServer`:L17
- `Pilot.LanguageServer/Pilot.LanguageServer.csproj` (15 L)
- `Pilot.LanguageServer/Program.cs` (53 L) — class `Program`:L10
- `Pilot.LanguageServer/Semantics/PilotDiagnosticCatalog.cs` (340 L) — class `PilotDiagnosticDescriptor`:L9, class `PilotDiagnosticCatalog`:L54
- `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (5217 L) — class `PilotSemanticAnalyzer`:L10, class `ScopeRestorer`:L5134, class `LspErrorListener`:L5146
- `Pilot.LanguageServer/Semantics/PilotSymbol.cs` (393 L) — enum `PilotSymbolKind`:L7, enum `PilotDiagnosticSeverity`:L35, class `PilotCodeAction`:L43, class `PilotDiagnostic`:L55, class `PilotSemanticTokenLegend`:L68, class `PilotSemanticToken`:L142, class `PilotSymbol`:L151, class `PilotSymbolReference`:L256, class `PilotScope`:L264, class `PilotAnalysisResult`:L310

### Pilot.Tests
- `Pilot.Tests/LspMessageHelper.cs` (13 L) — class `LspMessageHelper`:L6
- `Pilot.Tests/Pilot.Tests.csproj` (27 L)
- `Pilot.Tests/PilotAdtAndExhaustivenessTests.cs` (720 L) — class `PilotAdtAndExhaustivenessTests`:L33
- `Pilot.Tests/PilotAdvancedFeaturesTests.cs` (263 L) — class `PilotAdvancedFeaturesTests`:L11
- `Pilot.Tests/PilotAdvancedTypeInferenceTests.cs` (412 L) — class `PilotAdvancedTypeInferenceTests`:L18
- `Pilot.Tests/PilotBurstAndNativeTypesTests.cs` (150 L) — class `PilotBurstAndNativeTypesTests`:L14
- `Pilot.Tests/PilotCallQuotingCoreTests.cs` (219 L) — class `PilotCallQuotingCoreTests`:L24
- `Pilot.Tests/PilotCompilationAndScriptingTests.cs` (174 L) — class `PilotCompilationAndScriptingTests`:L11
- `Pilot.Tests/PilotComprehensiveKeywordValidationTests.cs` (452 L) — class `PilotComprehensiveKeywordValidationTests`:L13
- `Pilot.Tests/PilotContextContractsPatternsTests.cs` (297 L) — class `PilotContextContractsPatternsTests`:L11
- `Pilot.Tests/PilotContractDelegationAndStealingTests.cs` (234 L) — class `PilotContractDelegationAndStealingTests`:L10
- `Pilot.Tests/PilotDebuggingTests.cs` (265 L) — class `PilotDebuggingTests`:L12
- `Pilot.Tests/PilotDynamicForExpressionTests.cs` (566 L) — class `PilotDynamicForExpressionTests`:L27
- `Pilot.Tests/PilotErrorSystemTests.cs` (464 L) — class `PilotErrorSystemTests`:L22
- `Pilot.Tests/PilotExceptionProtectionsTests.cs` (196 L) — class `PilotExceptionProtectionsTests`:L11
- `Pilot.Tests/PilotExecutionEngineTests.cs` (260 L) — class `PilotExecutionEngineTests`:L9
- `Pilot.Tests/PilotExpressionErgonomicsTests.cs` (339 L) — class `PilotExpressionErgonomicsTests`:L16
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
- `Pilot.Tests/PilotSemanticAndLspTests.cs` (1088 L) — class `PilotSemanticAndLspTests`:L13, class `MathHelper`:L305
- `Pilot.Tests/PilotSemanticCallProjectionsTests.cs` (284 L) — class `PilotSemanticCallProjectionsTests`:L22
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
- `Runtime/Builtins/PilotBuiltinRegistry.cs` (1155 L) — class `PilotBuiltinRegistry`:L15
- `Runtime/Burst/PilotBurstEngine.cs` (145 L) — class `PilotBurstEngine`:L19
- `Runtime/Bytecode/PilotBytecodeChunk.cs` (586 L) — class `PilotJumpTable`:L12, class `PilotSerializedFieldDescriptor`:L29, class `PilotSerializedVariantDescriptor`:L38, class `PilotBytecodeTypeDescriptor`:L48, class `PilotBytecodeChunk`:L62
- `Runtime/Bytecode/PilotInstruction.cs` (46 L) — struct `PilotInstruction`:L11
- `Runtime/Bytecode/PilotOpcode.cs` (266 L) — enum `PilotOpcode`:L11
- `Runtime/Bytecode/PilVirtualMachine.cs` (1530 L) — class `PilVirtualMachine`:L20
- `Runtime/Concurrency/PilotFuture.cs` (72 L) — class `PilotFuture`:L11
- `Runtime/Contracts/PilotContractRegistry.cs` (313 L) — class `PilotContractRegistry`:L14
- `Runtime/ControlFlow/PilotControlFlowSignals.cs` (99 L) — enum `PilotFlowSignalKind`:L9, class `PilotControlFlowSignal`:L21, class `PilotReturnSignal`:L29, class `PilotBreakSignal`:L42, class `PilotContinueSignal`:L55, class `PilotYieldSignal`:L68, class `PilotRuntimeException`:L81
- `Runtime/ControlFlow/PilotExceptions.cs` (123 L) — class `PilotTypeException`:L9, class `PilotDivideByZeroException`:L18, class `PilotIndexOutOfRangeException`:L27, class `PilotKeyNotFoundException`:L36, class `PilotNullReferenceException`:L45, class `PilotAssertionException`:L54, class `PilotRecursionException`:L63, class `PilotContractException`:L72, class `PilotTimeoutException`:L81, class `PilotPatternMatchingException`:L90, class `PilotContextException`:L99, class `PilotUserException`:L108
- `Runtime/Effects/PilotEffectDryRunEngine.cs` (767 L) — enum `EffectDiffKind`:L19, class `PilotEffectDiff`:L33, class `ShadowEffectContext`:L96, class `PilotEffectPlan`:L124, class `PilotEffectDryRunEngine`:L236
- `Runtime/Environment/PilotBinding.cs` (22 L) — class `PilotBinding`:L8
- `Runtime/Environment/PilotEnvironment.cs` (59 L) — class `PilotEnvironment`:L9
- `Runtime/Environment/PilotScope.cs` (166 L) — class `PilotScope`:L10
- `Runtime/Execution/IPilotEngine.cs` (56 L) — interface `IPilotEngine`:L10
- `Runtime/Execution/PilotEngine.cs` (650 L) — class `PilotEngine`:L19, class `GeneratorRunner`:L351, class `MergedTextWriter`:L614
- `Runtime/Execution/PilotExecutionContext.cs` (155 L) — class `PilotExecutionContext`:L13
- `Runtime/Execution/PilotExecutionOptions.cs` (87 L) — class `PilotExecutionOptions`:L13
- `Runtime/Execution/PilotExecutionResult.cs` (57 L) — class `PilotExecutionResult`:L10
- `Runtime/Execution/PilotInterpreterVisitor.cs` (5739 L) — class `PilotInterpreterVisitor`:L19, struct `FStringSegment`:L41, enum `TargetAssignmentKind`:L57, class `TargetAssignmentInfo`:L66
- `Runtime/Execution/PilotPatternMatcher.cs` (986 L) — class `PilotPatternMatcher`:L16
- `Runtime/Extensions/PilotLexicalExtension.cs` (138 L) — class `PilotLexicalExtension`:L11
- `Runtime/Generative/PilotClassNode.cs` (127 L) — class `PilotFieldNode`:L11, class `PilotClassNode`:L38
- `Runtime/HotReload/PilotHotReloader.cs` (732 L) — class `PilotSymbolSnapshot`:L19, class `PilotPatchRecord`:L165, class `PilotPatchResult`:L208, class `PilotHotReloadSession`:L242
- `Runtime/Interop/IPolyglotProvider.cs` (31 L) — interface `IPolyglotProvider`:L10
- `Runtime/Interop/IPolyglotTypeConverter.cs` (20 L) — interface `IPolyglotTypeConverter`:L9
- `Runtime/Interop/PolyglotRegistry.cs` (1572 L) — class `PolyglotRegistry`:L18, class `DefaultPythonProvider`:L78, class `DefaultCSharpProvider`:L160, class `DefaultRustProvider`:L236, class `DefaultJavaScriptProvider`:L283, class `DefaultJavaProvider`:L341, class `DefaultAsmProvider`:L393, class `DefaultCProvider`:L543, class `DefaultCppProvider`:L592, class `DefaultLuaProvider`:L640, class `DefaultPostgreSqlProvider`:L690, class `DefaultMySqlProvider`:L764, class `PolyglotSharedMemoryInterop`:L788
- `Runtime/Interop/PolyglotTypeBridge.cs` (490 L) — class `PolyglotTypeBridge`:L16
- `Runtime/NativeMemory/PilotLoopAbiTypes.cs` (226 L) — struct `RangeWindow`:L15, struct `SliceBuffer`:L48, struct `UnmanagedWorklist`:L146
- `Runtime/NativeMemory/PilotNativeArray.cs` (212 L) — class `PilotNativeArray`:L15
- `Runtime/NativeMemory/PilotNativeMap.cs` (281 L) — class `PilotNativeMap`:L15
- `Runtime/NativeMemory/PilotSharedBuffer.cs` (750 L) — enum `PilotSharedElementKind`:L14, class `PilotSharedBuffer`:L28
- `Runtime/Packaging/Adapters/BaseInteropPackageAdapter.cs` (218 L) — class `BaseInteropPackageAdapter`:L12, class `NuGetPackageAdapter`:L140, class `NpmPackageAdapter`:L156, class `PipPackageAdapter`:L171, class `CargoPackageAdapter`:L186, class `MavenPackageAdapter`:L200, class `LuaRocksPackageAdapter`:L205
- `Runtime/Packaging/IInteropPackageAdapter.cs` (31 L) — interface `IInteropPackageAdapter`:L10
- `Runtime/Packaging/PilotPackageLock.cs` (65 L) — class `PilotPackageLock`:L12, class `LockedPackageEntry`:L49
- `Runtime/Packaging/PilotPackageManager.cs` (376 L) — class `PilotPackageManager`:L17
- `Runtime/Packaging/PilotPackageManifest.cs` (100 L) — class `PilotPackageManifest`:L14
- `Runtime/Reactive/PilotSignalGraph.cs` (812 L) — class `PilotSignalNode`:L18, class `PilotSourceSignal`:L153, class `PilotComputedSignal`:L233, class `PilotSignalGraph`:L284, class `TopologicalRankComparer`:L286, class `ObjectPropertyKey`:L300, class `ScopeVariableKey`:L322
- `Runtime/SelectiveExecution/PilotBenchmarkReport.cs` (47 L) — class `PilotBenchmarkResult`:L9, class `PilotBenchmarkReport`:L37
- `Runtime/SelectiveExecution/PilotSelectiveExecutionEngine.cs` (323 L) — class `PilotSelectiveExecutionEngine`:L20
- `Runtime/SelectiveExecution/PilotTestReport.cs` (46 L) — class `PilotTestCaseResult`:L9, class `PilotTestReport`:L32
- `Runtime/Serialization/PilotJsonSerializer.cs` (198 L) — class `PilotJsonSerializer`:L15
- `Runtime/Values/PilotAdt.cs` (1037 L) — class `PilotAdtFieldDefinition`:L14, record `PilotFieldMemoryLayout`:L35, record `PilotVariantMemoryLayout`:L46, class `PilotAdtMemoryLayout`:L58, class `PilotAdtFloatSlot`:L563, class `PilotAdtVariantDefinition`:L572, class `PilotAdtDefinition`:L721, class `PilotAdtInstance`:L768, class `PilotRecordDefinition`:L828, class `PilotRecordInstance`:L951, class `PilotKeywordVariant`:L1002
- `Runtime/Values/PilotFunction.cs` (314 L) — class `PilotParameter`:L12, class `PilotFunction`:L37
- `Runtime/Values/PilotGenerator.cs` (123 L) — enum `PilotGeneratorState`:L13, class `PilotGenerator`:L25
- `Runtime/Values/PilotKeyword.cs` (301 L) — class `PilotKeyword`:L13
- `Runtime/Values/PilotLazy.cs` (55 L) — class `PilotLazy`:L9
- `Runtime/Values/PilotMatrix.cs` (209 L) — class `PilotMatrix`:L9
- `Runtime/Values/PilotMethodGroup.cs` (635 L) — class `PilotMethodGroup`:L21
- `Runtime/Values/PilotObject.cs` (665 L) — class `PilotClassDefinition`:L11, class `PilotObject`:L393
- `Runtime/Values/PilotRange.cs` (98 L) — class `PilotRange`:L10
- `Runtime/Values/PilotValue.cs` (2330 L) — struct `PilotValue`:L15, class `PilotNestedSomeSentinel`:L27, class `PilotFailurePrimitiveSentinel`:L32, class `PilotNestedFailureBox`:L47
- `Runtime/Values/PilotValueKind.cs` (39 L) — enum `PilotValueKind`:L6
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
- `tests/31_expression_ergonomics_and_extensions.pilot` (189 L)
- `tests/32_dynamic_for_expressions_and_c_abi.pilot` (69 L)
- `tests/33_advanced_type_inference.pilot` (55 L)
- `tests/34_keywords_and_call_quoting.pilot` (76 L)
- `tests/35_semantic_call_projections.pilot` (85 L)

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
- `vscode-pilot/src/extension.ts` (711 L)
- `vscode-pilot/syntaxes/pilot.tmLanguage.json` (405 L)
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
