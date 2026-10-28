# Pilot — AI-Only Codebase Navigation & Symbol Index

> **STRICTLY FOR AI AGENTS**: Do not link this file in user-facing documentation. Use exact paths and `:L<line>` anchors below with `read_file` (`StartLine`/`EndLine`) to jump directly to code without exploratory directory/grep bouncing.
> **Auto-Sync Stamp**: `21EF9DA23F689B1B19FCD936053775432989E3DF9C0FA145E77D45C7F91DEC5C` | **Total Indexed Files**: 282 | **Regenerate**: `pwsh -NoProfile -File .agents/scripts/update_ai_index.ps1`

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
| **Ownership, Units & Contracts** | `Analysis/Ownership/PilotBorrowChecker.cs`, `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs`, `Analysis/ContractVerification/PilotContractVerifier.cs` | Runtime contracts registry in `Runtime/Contracts/PilotContractRegistry.cs`. |
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

Total lines in `antlr/Pilot.g4`: **1653**

### Parser Rules (232)
`program`:L15 · `identifier`:L23 · `stmt`:L53 · `simple_stmt`:L58 · `small_stmt`:L62 · `compound_stmt`:L90
`suite`:L141 · `guard_stmt`:L151 · `pattern_guard`:L155 · `defer_stmt`:L160 · `type_annotation`:L168
`return_type`:L172 · `type_spec`:L176 · `union_type`:L180 · `intersection_type`:L184 · `primary_type`:L188
`base_type`:L192 · `type_arguments`:L202 · `generic_params`:L206 · `generic_param`:L210 · `where_type_clause`:L215
`where_type_item`:L219 · `access_modifier`:L224 · `declaration_modifier`:L231 · `binding_pattern`:L241
`tuple_pattern`:L249 · `list_pattern`:L253 · `record_pattern`:L257 · `binding_pattern_item`:L261
`record_pattern_field`:L265 · `pattern_assign_stmt`:L270 · `let_stmt`:L274 · `var_stmt`:L278 · `const_stmt`:L282
`ref_stmt`:L286 · `dynamic_stmt`:L290 · `type_alias_stmt`:L294 · `adt_sum_body`:L299 · `adt_variant`:L305
`adt_variant_payload`:L309 · `adt_field_list`:L313 · `adt_field`:L317 · `annassign_stmt`:L322 · `expr_stmt`:L326
`channel_send_stmt`:L330 · `augassign`:L334 · `func_def`:L354 · `contract_clause`:L358 · `parameters`:L363
`parameter`:L367 · `class_def`:L371 · `state_def`:L376 · `class_bases`:L380 · `struct_def`:L385 · `record_def`:L390
`struct_member`:L395 · `interface_def`:L400 · `enum_def`:L405 · `enum_suite`:L409 · `enum_member`:L413
`actor_def`:L418 · `model_def`:L423 · `property_def`:L432 · `property_body`:L436 · `property_accessor`:L440
`get_accessor`:L445 · `set_accessor`:L449 · `operator_def`:L454 · `overloadable_op`:L458 · `extension_def`:L466
`adapter_def`:L471 · `macro_def`:L476 · `quote_stmt`:L480 · `quote_expr`:L484 · `quasiquote_expr`:L488
`splice_expr`:L492 · `extern_def`:L499 · `extern_member`:L503 · `foreign_language`:L509
`inline_foreign_block`:L513 · `decorators`:L518 · `decorator`:L522 · `labeled_stmt`:L530 · `block`:L534
`if_stmt`:L538 · `while_stmt`:L542 · `for_stmt`:L546 · `for_target`:L550 · `match_stmt`:L555 · `match_suite`:L559
`match_arm`:L563 · `pattern`:L567 · `or_pattern`:L571 · `and_pattern`:L575 · `not_pattern`:L579
`primary_pattern`:L585 · `range_pattern`:L598 · `range_bound`:L602 · `signed_number_pattern`:L607
`keyword_pattern`:L611 · `pattern_arg`:L615 · `relational_pattern`:L620 · `collection_pattern`:L624
`slice_pattern_element`:L628 · `property_pattern`:L633 · `property_pattern_member`:L637
`tuple_pattern_match`:L641 · `type_or_var_pattern`:L645 · `generic_pattern_suffix`:L650 · `try_stmt`:L654
`except_clause`:L658 · `with_stmt`:L662 · `with_item`:L666 · `flow_stmt`:L672 · `pass_stmt`:L680
`assert_stmt`:L684 · `steal_stmt`:L688 · `delegate_stmt`:L692 · `contract_forward_stmt`:L696 · `global_stmt`:L700
`import_stmt`:L704 · `foreign_import_stmt`:L710 · `foreign_target`:L715 · `import_name`:L719 · `import_from`:L723
`import_as_name`:L727 · `dotted_as_name`:L731 · `import_as_names`:L735 · `dotted_as_names`:L739
`dotted_name`:L743 · `deadline_stmt`:L751 · `within_stmt`:L755 · `periodic_stmt`:L759 · `noalloc_stmt`:L763
`region_stmt`:L767 · `duration_term`:L771 · `spawn_stmt`:L780 · `atomic_stmt`:L784 · `sync_stmt`:L788
`isolate_stmt`:L792 · `select_stmt`:L796 · `select_suite`:L800 · `select_arm`:L804 · `select_case_target`:L810
`dsl_stmt`:L820 · `dsl_args`:L824 · `unit_stmt`:L828 · `unit_expr`:L832 · `unit_term`:L836 · `database_stmt`:L847
`database_def`:L851 · `transaction_stmt`:L856 · `savepoint_stmt`:L860 · `savepoint_block`:L864 · `commit_stmt`:L868
`rollback_stmt`:L872 · `insert_stmt`:L877 · `insert_values_group`:L884 · `on_conflict_clause`:L888
`update_stmt`:L893 · `set_assignment_list`:L900 · `set_assignment`:L904 · `delete_stmt`:L909
`returning_clause`:L915 · `index_clause`:L920 · `index_field`:L924 · `index_type`:L928 · `index_hint`:L932
`query_expr`:L937 · `from_clause`:L941 · `join_clause`:L945 · `where_clause`:L949 · `group_clause`:L953
`order_clause`:L957 · `ordering_term`:L961 · `select_clause`:L965 · `select_projection`:L969 · `select_item`:L974
`limit_clause`:L978 · `sql_query_expr`:L983 · `from_source`:L987 · `lookup_expr`:L992 · `set_op`:L997
`testlist`:L1008 · `exprlist`:L1012 · `test`:L1016 · `lambda_expr`:L1021 · `lambda_body`:L1030
`capture_list`:L1035 · `capture_item`:L1039 · `lambda_params`:L1045 · `lambda_param`:L1050 · `pipe_expr`:L1055
`set_query_expr`:L1060 · `coalesce_expr`:L1067 · `or_test`:L1071 · `and_test`:L1075 · `not_test`:L1079
`comparison`:L1084 · `comp_op`:L1088 · `range_expr`:L1101 · `bitor_expr`:L1105 · `bitxor_expr`:L1109
`bitand_expr`:L1113 · `shift_expr`:L1117 · `arith_expr`:L1121 · `term`:L1125 · `factor`:L1129 · `unary_op`:L1135
`power`:L1145 · `atom_expr`:L1149 · `trailer`:L1153 · `cascade_trailer`:L1166 · `cascade_target`:L1170
`over_clause`:L1177 · `subscriptlist`:L1181 · `subscript`:L1185 · `atom`:L1193 · `collection_expr`:L1216
`collection_element`:L1220 · `nameof_expr`:L1224 · `macro_call`:L1228 · `symbol_literal`:L1232 · `literal`:L1237
`testlist_comp`:L1256 · `dictorsetmaker`:L1260 · `dict_entry`:L1265 · `comp_for`:L1270 · `comp_iter`:L1274
`comp_if`:L1279 · `arglist`:L1283 · `argument`:L1287

### Lexer Tokens (252)
`PUBLIC`:L1300 · `PRIVATE`:L1301 · `PROTECTED`:L1302 · `INTERNAL`:L1303 · `STATIC`:L1304 · `PARTIAL`:L1305
`STEAL`:L1306 · `CONTRACTS`:L1307 · `DELEGATE`:L1308 · `PARAMS`:L1309 · `ARGS`:L1310 · `KWARGS`:L1311
`NAMEOF`:L1312 · `EACH`:L1313 · `REPEAT`:L1314 · `ADAPTER`:L1315 · `CONVERTER`:L1316 · `DEF`:L1318 · `FN`:L1319
`CLASS`:L1320 · `STRUCT`:L1321 · `RECORD`:L1322 · `INTERFACE`:L1323 · `PROTOCOL`:L1324 · `ENUM`:L1325
`ACTOR`:L1326 · `LET`:L1327 · `VAR`:L1328 · `CONST`:L1329 · `TYPE`:L1330 · `REF`:L1331 · `DYNAMIC`:L1332
`SYMBOL`:L1333 · `PURE`:L1334 · `MUT`:L1335 · `IMMUT`:L1336 · `PROPERTY`:L1337 · `GET`:L1338 · `OPERATOR`:L1339
`EXTENSION`:L1340 · `STATE`:L1341 · `SELECT`:L1344 · `FROM`:L1345 · `WHERE`:L1346 · `JOIN`:L1347 · `INNER`:L1348
`LEFT`:L1349 · `RIGHT`:L1350 · `FULL`:L1351 · `CROSS`:L1352 · `OUTER`:L1353 · `ON`:L1354 · `GROUP`:L1355
`BY`:L1356 · `HAVING`:L1357 · `ORDER`:L1358 · `ASC`:L1359 · `DESC`:L1360 · `ASCENDING`:L1361 · `DESCENDING`:L1362
`LIMIT`:L1363 · `OFFSET`:L1364 · `INTO`:L1365 · `DISTINCT`:L1366 · `UNION`:L1367 · `ALL`:L1368 · `INTERSECT`:L1369
`EXCEPT`:L1370 · `DIFFERENCE`:L1371 · `INDEX`:L1372 · `INDEXED`:L1373 · `LOOKUP`:L1374 · `TABLE`:L1375
`SCHEMA`:L1376 · `INSERT`:L1377 · `UPDATE`:L1378 · `DELETE`:L1379 · `VALUES`:L1380 · `SET_KW`:L1381
`RETURNING`:L1382 · `CONFLICT`:L1383 · `DO`:L1384 · `NOTHING`:L1385 · `OVER`:L1386 · `PARTITION`:L1387
`WINDOW`:L1388 · `TRANSACTION`:L1389 · `COMMIT`:L1390 · `ROLLBACK`:L1391 · `SAVEPOINT`:L1392 · `TO`:L1393
`CONNECT`:L1394 · `DATABASE`:L1395 · `MODEL`:L1396 · `ASYNC`:L1399 · `AWAIT`:L1400 · `SPAWN`:L1401
`PARALLEL`:L1402 · `CHAN`:L1403 · `CHANNEL`:L1404 · `ATOMIC`:L1405 · `SYNC`:L1406 · `LOCK`:L1407 · `ISOLATE`:L1408
`REALTIME`:L1411 · `DEADLINE`:L1412 · `ON_MISSED`:L1413 · `WITHIN`:L1414 · `FALLBACK`:L1415 · `PERIOD`:L1416
`PERIODIC`:L1417 · `PRIORITY`:L1418 · `JITTER`:L1419 · `BOUNDED`:L1420 · `NOALLOC`:L1421 · `STACKALLOC`:L1422
`REGION`:L1423 · `WCET`:L1424 · `HARD_RT`:L1425 · `SOFT_RT`:L1426 · `EXTERN`:L1429 · `CSHARP`:L1430
`PYTHON`:L1431 · `RUST`:L1432 · `JS`:L1433 · `JAVASCRIPT`:L1434 · `JAVA`:L1435 · `ASM`:L1436 · `ASSEMBLY`:L1437
`CPP`:L1438 · `C_KW`:L1439 · `LUA`:L1440 · `SQL_KW`:L1441 · `POSTGRES`:L1442 · `POSTGRESQL`:L1443 · `MYSQL`:L1444
`MODULE`:L1445 · `LIB`:L1446 · `DSL`:L1449 · `DOMAIN`:L1450 · `UNIT`:L1451 · `MACRO`:L1452 · `DEFMACRO`:L1453
`QUOTE`:L1454 · `QUASIQUOTE`:L1455 · `SPLICE`:L1456 · `UNQUOTE`:L1457 · `UNQUOTE_SPLICING`:L1458 · `MATCH`:L1459
`CASE`:L1460 · `WHEN`:L1461 · `DEFAULT`:L1462 · `TIMEOUT`:L1463 · `GUARD`:L1466 · `DEFER`:L1467 · `CONTEXT`:L1468
`REQUIRES`:L1469 · `ENSURES`:L1470 · `IF`:L1471 · `ELIF`:L1472 · `ELSE`:L1473 · `WHILE`:L1474 · `FOR`:L1475
`IN`:L1476 · `RETURN`:L1477 · `YIELD`:L1478 · `BREAK`:L1479 · `CONTINUE`:L1480 · `PASS`:L1481 · `RAISE`:L1482
`TRY`:L1483 · `FINALLY`:L1484 · `WITH`:L1485 · `AS`:L1486 · `ASSERT`:L1487 · `IMPORT`:L1488 · `GLOBAL`:L1489
`LAMBDA`:L1490 · `AND`:L1491 · `OR`:L1492 · `NOT`:L1493 · `IS`:L1494 · `TRUE`:L1497 · `FALSE`:L1498 · `NONE`:L1499
`NULL`:L1500 · `UNDEFINED`:L1501 · `JSON_GET_TEXT`:L1504 · `ARROW_LEFT`:L1505 · `ARROW`:L1506 · `FAT_ARROW`:L1507
`PIPE_FORWARD`:L1508 · `PIPE_BACKWARD`:L1509 · `SAFE_CASCADE`:L1510 · `SAFE_NAV`:L1511 · `COALESCE_EQUAL`:L1512
`COALESCE`:L1513 · `JSON_CONTAINS`:L1514 · `JSON_CONTAINED`:L1515 · `ELLIPSIS`:L1516 · `DOTDOT_EQUAL`:L1517
`DOTDOT_LESS`:L1518 · `DOTDOT`:L1519 · `DOUBLESTAR`:L1520 · `DOUBLESLASH`:L1521 · `EQEQUAL`:L1522
`NOTEQUAL`:L1523 · `LESSEQUAL`:L1524 · `GREATEREQUAL`:L1525 · `LEFTSHIFT`:L1526 · `RIGHTSHIFT`:L1527
`PLUSEQUAL`:L1528 · `MINEQUAL`:L1529 · `STAREQUAL`:L1530 · `SLASHEQUAL`:L1531 · `PERCENTEQUAL`:L1532
`AMPEREQUAL`:L1533 · `VBAREQUAL`:L1534 · `CIRCUMFLEXEQUAL`:L1535 · `LEFTSHIFTEQUAL`:L1536 · `RIGHTSHIFTEQUAL`:L1537
`DOUBLESTAREQUAL`:L1538 · `DOUBLESLASHEQUAL`:L1539 · `WALRUS`:L1540 · `LPAR`:L1543 · `RPAR`:L1544 · `LSQB`:L1545
`RSQB`:L1546 · `LBRACE`:L1547 · `RBRACE`:L1548 · `COLON`:L1549 · `SEMI`:L1550 · `COMMA`:L1551 · `DOT`:L1552
`AT`:L1553 · `PLUS`:L1554 · `MINUS`:L1555 · `STAR`:L1556 · `SLASH`:L1557 · `PERCENT`:L1558 · `VBAR`:L1559
`AMPER`:L1560 · `CIRCUMFLEX`:L1561 · `TILDE`:L1562 · `LESS`:L1563 · `GREATER`:L1564 · `EQUAL`:L1565
`QUESTION`:L1566 · `EXCLAMATION`:L1567 · `DOLLAR`:L1568 · `BACKQUOTE`:L1569 · `UNDERSCORE`:L1570

---

## 3. Large File Internal Method Maps (Files > 350 Lines)

Use these `:L<line>` method anchors to pass exact `StartLine`/`EndLine` into `read_file` for large files:

### `Analysis/Ownership/PilotBorrowChecker.cs` (503 lines)
`PushScope`:L84 · `PopScope`:L89 · `CheckRootScopeExit`:L112 · `RegisterResource`:L134 · `TryLookupResource`:L143
`VisitProgram`:L157 · `VisitStruct_def`:L171 · `VisitClass_def`:L188 · `VisitLet_stmt`:L205 · `VisitVar_stmt`:L245
`VisitFunc_def`:L284 · `VisitExpr_stmt`:L308 · `CheckRhsForMove`:L339 · `CheckExpressionUsage`:L435
`CheckZeroAllocation`:L471 · `GetExactText`:L487 · `HasDecorator`:L498

### `Analysis/PatternMatching/PilotExhaustivenessChecker.cs` (1800 lines)
`CheckProgram`:L45 · `CollectDeclarations`:L62 · `RegisterTypeAliasAdt`:L91 · `RegisterEnumAdt`:L132
`RegisterRecord`:L147 · `RegisterStructAsRecord`:L155 · `RegisterClassTypestates`:L164 · `StoreAdt`:L182
`RefreshLayouts`:L191 · `ExtractGenericParams`:L199 · `ExtractFields`:L202 · `VisitFunc_def`:L233
`VisitLet_stmt`:L260 · `VisitVar_stmt`:L272 · `VisitMatch_stmt`:L284 · `ResolveDeclType`:L290
`InferScrutineeType`:L299 · `TryExtractAtom`:L376 · `ParseRichType`:L385 · `OrPat`:L463 · `CtorPat`:L464
`CtorDesc`:L483 · `VerifyMatch`:L491 · `RefineTypeFromPatterns`:L577 · `TryInferTypeFromSinglePat`:L588
`TryEvaluateStaticBoolGuard`:L618 · `LowerPattern`:L627 · `LowerOrPattern`:L633 · `LowerAndPattern`:L640
`LowerNotPattern`:L666 · `NegatePattern`:L683 · `LowerPrimaryPattern`:L722 · `LowerRangePattern`:L789
`ParseBoundInt`:L817 · `ParseUnsignedLong`:L823 · `LowerLiteralPattern`:L832 · `LowerRelationalPattern`:L858
`LowerCollectionPattern`:L874 · `LowerPropertyPattern`:L916 · `LowerTypeOrVarPattern`:L941
`SubstituteGenericType`:L1094 · `MakeIntInterval`:L1113 · `GetIntDomainBounds`:L1116 · `IsUseful`:L1134
`FindMissingWitnesses`:L1232 · `FindWitnessVector`:L1265 · `CheckSignatureCompleteness`:L1327
`GetFullClosedDomainConstructors`:L1341 · `SpecializeMatrix`:L1467 · `DefaultMatrix`:L1537
`ExpandColumnZeroOrs`:L1550 · `ExpandOrRow`:L1567 · `PartitionInterval`:L1585 · `ComputeListSplitBound`:L1615
`ExpandSliceRowArgs`:L1631 · `ExpandSliceQueryArgs`:L1646 · `GetCtorSubTypes`:L1649 · `GetListElementType`:L1676
`FormatConstructorWitness`:L1681 · `FormatIntWitness`:L1701 · `FormatInterval`:L1711 · `PrependWildcards`:L1722
`ConcatPatterns`:L1731 · `ConcatTypes`:L1740 · `PrependRepeated`:L1749 · `IsIntegerType`:L1758
`IsBuiltinTypeName`:L1761 · `StripTypeArgs`:L1767 · `StripQuotes`:L1773 · `SplitTopLevel`:L1776

### `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (522 lines)
`ToString`:L29 · `RegisterBuiltins`:L54 · `PushScope`:L70 · `AggregateResult`:L77 · `VisitProgram`:L82
`PopScope`:L95 · `SetVariableType`:L103 · `TryLookupType`:L112 · `ParseTypeString`:L131 · `VisitLet_stmt`:L199
`VisitVar_stmt`:L227 · `VisitFunc_def`:L255 · `VisitTest`:L285 · `VisitArith_expr`:L295 · `VisitAtom`:L320
`VisitAtom_expr`:L371 · `VisitType_alias_stmt`:L407 · `VisitEnum_def`:L446 · `VisitRecord_def`:L462
`RegisterAdtVariantType`:L479 · `ExtractFieldTypes`:L501

### `Cli/Program.cs` (1172 lines)
`Main`:L20 · `ExecuteInlineScript`:L251 · `ExecuteBuildSubcommand`:L264 · `ExecuteCompilationSubcommand`:L380
`RunScriptFile`:L466 · `ExecuteTestSubcommand`:L577 · `ExecuteBenchmarkSubcommand`:L680
`ExecutePatchSubcommand`:L740 · `ExecuteWatchSubcommand`:L822 · `ExecutePackageSubcommand`:L867
`ExecuteVenvSubcommand`:L961 · `PrintUsage`:L1028 · `ParseAndReport`:L1063

### `CodeGen/Engine/PilotCodeGeneratorPipeline.cs` (410 lines)
`ShouldRun`:L39 · `Run`:L59 · `Run`:L73 · `ExtractScriptModel`:L165 · `ExtractClassModel`:L212
`ExtractMethodModel`:L287 · `ExtractDecorator`:L326 · `ExtractViaRegexFallback`:L352
`ExecuteInScriptGenerators`:L381

### `Compiler/PilotBytecodeCompiler.cs` (2162 lines)
`IsImmutable`:L41 · `AllocateTempSlot`:L48 · `RegisterBindingMetadata`:L51 · `HasDeclaredBinding`:L71
`EmitStoreBinding`:L82 · `EmitRestoreBinding`:L97 · `EmitLoadBindingOrUndefined`:L110 · `EmitLiteralConstant`:L123
`TryResolveAdtVariant`:L145 · `TryResolveAdtType`:L192 · `TryResolveRecordType`:L207
`IsKnownClassOrStructName`:L222 · `Compile`:L247 · `CompileSuite`:L259 · `PrescanProgramTypeDeclarations`:L298
`RegisterAdtCompileTime`:L338 · `RegisterRecordCompileTime`:L347 · `TryBuildTypeAliasAdtDescriptor`:L352
`BuildEnumDescriptor`:L440 · `BuildRecordDescriptor`:L468 · `ExtractGenericParamList`:L486
`ExtractSerializedFields`:L492 · `VisitProgram`:L529 · `VisitStmt`:L543 · `VisitSimple_stmt`:L552
`VisitSmall_stmt`:L567 · `VisitCompound_stmt`:L582 · `VisitType_alias_stmt`:L608 · `VisitEnum_def`:L629
`VisitRecord_def`:L647 · `VisitMatch_stmt`:L665 · `VisitGuard_stmt`:L671 · `VisitSuite`:L677 · `VisitLet_stmt`:L699
`VisitVar_stmt`:L721 · `VisitConst_stmt`:L743 · `ExtractBindingIdentifiers`:L765
`CompileVariableDeclaration`:L811 · `VisitExpr_stmt`:L837 · `AssignToTarget`:L881 · `CompileAugmentedAssign`:L947
`EmitAugassignOp`:L1064 · `VisitFlow_stmt`:L1082 · `VisitIf_stmt`:L1128 · `VisitWhile_stmt`:L1181
`VisitFor_stmt`:L1230 · `VisitFunc_def`:L1289 · `VisitTest`:L1327 · `VisitPipe_expr`:L1354
`VisitSet_query_expr`:L1371 · `VisitCoalesce_expr`:L1380 · `VisitOr_test`:L1399 · `VisitAnd_test`:L1416
`VisitNot_test`:L1433 · `VisitComparison`:L1445 · `VisitRange_expr`:L1483 · `VisitBitor_expr`:L1500
`VisitBitxor_expr`:L1514 · `VisitBitand_expr`:L1528 · `VisitShift_expr`:L1542 · `VisitArith_expr`:L1557
`VisitTerm`:L1572 · `VisitFactor`:L1594 · `VisitPower`:L1624 · `VisitAtom_expr`:L1637
`RequiresAtomExprAstFallback`:L1710 · `HasNamedArguments`:L1753 · `TryResolveKnownFieldOrderForCall`:L1764
`OrderCallArgumentExpressions`:L1804 · `VisitAtom`:L1852 · `VisitLiteral`:L1972 · `VisitCollection_expr`:L2065
`IsSimpleIdentifier`:L2098 · `GetAtomExpr`:L2114 · `CompileAtomExprPrefix`:L2122 · `StripQuotes`:L2147

### `Compiler/PilotCompiledProgram.cs` (559 lines)
`Run`:L70 · `Run`:L75 · `RunWithArgs`:L116 · `ExecuteCore`:L121 · `Save`:L207 · `Save`:L234 · `ToBytes`:L245
`Load`:L252 · `Load`:L314 · `FromBytes`:L320 · `ApplyHotPatch`:L337 · `ApplyPatchSource`:L347
`ApplyPatchFile`:L363 · `ApplyPatch`:L392 · `MergeProgramSources`:L409 · `GetDeclarationName`:L526
`ExtractStatementSource`:L539

### `Compiler/PilotMatchBytecodeLowerer.cs` (2009 lines)
`LowerMatch`:L71 · `AnalyzeArmBranches`:L162 · `ClassifyBranch`:L180 · `TryEmitAdtJumpTable`:L401
`TryEmitBoolJumpTable`:L491 · `TryEmitDenseIntJumpTable`:L551 · `EmitBranchChain`:L630 · `EmitArmBody`:L712
`SuiteLeavesExpressionOnStack`:L746 · `SimpleStmtLeavesExpressionOnStack`:L766 · `LowerIfStmt`:L781
`LowerWhileLetStmt`:L847 · `LowerGuardStmt`:L871 · `EmitCommitWithOptionalGuard`:L922
`FindBranchPatternGuard`:L969 · `LowerIsPatternExpr`:L986 · `EmitPatternTest`:L1023 · `EmitAndPatternTest`:L1060
`EmitNotPatternTest`:L1076 · `EmitPrimaryPatternTest`:L1112 · `EmitRangePatternTest`:L1227
`EmitKeywordPatternTest`:L1265 · `EmitRelationalPatternTest`:L1300 · `EmitTypeOrVarPatternTest`:L1335
`EmitOrderedFieldSubpatterns`:L1503 · `EmitCollectionSlicePatternTest`:L1541 · `EmitPropertyPatternTest`:L1631
`StageSlotValue`:L1674 · `CommitStagedBindings`:L1698 · `PredeclarePatternBindingsInCompiler`:L1707
`CollectBoundVariables`:L1716 · `CollectFromPattern`:L1723 · `CollectFromNotPattern`:L1736
`IsIrrefutableSubpattern`:L1821 · `IsBareOptionalBindingPattern`:L1859 · `TryGetSimpleWildcard`:L1862
`IsSpreadSliceElement`:L1870 · `FindFieldIndex`:L1873 · `FindBranchPatternAndMutability`:L1888
`ExtractKeywordText`:L1920 · `IsPrimitiveTypeName`:L1928 · `TryParseLiteralInt`:L1936 · `TryExtractIntRange`:L1945
`EvaluateRangeBoundValue`:L1965 · `ParseSignedNumberValue`:L1983 · `ParseUnsignedIntToken`:L1994

### `Native/CAbi/PilotCAbiEmitter.cs` (473 lines)
`EmitHeader`:L24 · `EmitHeader`:L43 · `SanitizeCIdentifier`:L179 · `Extract`:L215 · `PrescanTypes`:L246
`ExtractGenerics`:L323 · `ExtractAdtFields`:L326 · `VisitFunc_def`:L349 · `VisitStruct_def`:L375
`VisitClass_def`:L392 · `MapType`:L433

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

### `Pilot.LanguageServer/Lsp/PilotLspServer.cs` (743 lines)
`RunAsync`:L99 · `ProcessMessage`:L120 · `UpdateDocumentAndBuildDiagnostics`:L494 · `TryGetAnalysis`:L527
`ConvertToDocumentSymbol`:L544 · `GetCompletionItemKind`:L563 · `CreateRange`:L578 · `BuildResponse`:L584
`ExtractWordAt`:L595 · `SendRawJson`:L642 · `ReadMessageAsync`:L660

### `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (4373 lines)
`IsBuiltinType`:L48 · `Analyze`:L264 · `CheckUnusedVariables`:L334 · `PreRegisterTopLevelDeclarations`:L371
`RegisterFunctionStub`:L461 · `RegisterTypeStub`:L486 · `DeclareBindingPattern`:L510 · `VisitLet_stmt`:L569
`VisitVar_stmt`:L584 · `VisitConst_stmt`:L619 · `VisitRef_stmt`:L634 · `VisitDynamic_stmt`:L649
`VisitAnnassign_stmt`:L657 · `VisitType_alias_stmt`:L688 · `VisitPlainTypeAlias`:L697 · `VisitExpr_stmt`:L710
`CheckImmutableMemberAssignment`:L820 · `StripGenericSuffixForLookup`:L875 · `FindFirstAtomExpr`:L882
`VisitGuard_stmt`:L893 · `VisitDefer_stmt`:L941 · `VisitFunc_def`:L947 · `VisitContract_clause`:L1092
`VisitPattern_assign_stmt`:L1105 · `VisitClass_def`:L1114 · `VisitStruct_def`:L1146 · `VisitInterface_def`:L1175
`VisitEnum_def`:L1191 · `VisitActor_def`:L1218 · `VisitModel_def`:L1234 · `VisitProperty_def`:L1251
`VisitOperator_def`:L1321 · `VisitExtension_def`:L1358 · `VisitAdapter_def`:L1377 · `VisitUnit_stmt`:L1407
`VisitDsl_stmt`:L1423 · `VisitMacro_def`:L1439 · `VisitQuote_stmt`:L1470 · `VisitQuote_expr`:L1480
`VisitQuasiquote_expr`:L1489 · `VisitSplice_expr`:L1501 · `VisitForeign_import_stmt`:L1513
`VisitExtern_def`:L1608 · `ProcessExternMember`:L1671 · `ProcessExternFunction`:L1687 · `ProcessExternType`:L1763
`VisitInline_foreign_block`:L1817 · `VisitDatabase_stmt`:L1835 · `VisitTransaction_stmt`:L1846
`VisitSavepoint_stmt`:L1859 · `VisitSavepoint_block`:L1870 · `VisitRollback_stmt`:L1882 · `VisitNoalloc_stmt`:L1904
`VisitWhile_stmt`:L1914 · `VisitFor_stmt`:L1950 · `VisitIf_stmt`:L2018 · `VisitFlow_stmt`:L2038
`VisitPass_stmt`:L2134 · `VisitAssert_stmt`:L2141 · `VisitComparison`:L2148 · `VisitComp_op`:L2163
`VisitSelect_stmt`:L2171 · `VisitSelect_arm`:L2190 · `VisitQuery_expr`:L2214 · `VisitSql_query_expr`:L2263
`IsInsideSyncFunction`:L2291 · `VisitUnary_op`:L2308 · `VisitPrimary_type`:L2340 · `VisitAtom_expr`:L2375
`VisitTerm`:L2477 · `VisitLabeled_stmt`:L2503 · `VisitBlock`:L2513 · `VisitLambda_expr`:L2522
`VisitMacro_call`:L2606 · `VisitSymbol_literal`:L2618 · `VisitCascade_trailer`:L2631 · `VisitNameof_expr`:L2670
`VisitAtom`:L2689 · `InferLiteralType`:L2857 · `ValidateAndResolveType`:L2880 · `IsCompatibleType`:L2908
`IsStrictPrimitive`:L2991 · `NormalizePrimitive`:L2997 · `TryExtractUnit`:L3005 · `ExpandUnitExpression`:L3018
`SuiteGuaranteesExit`:L3029 · `SimpleStmtExits`:L3042 · `VisitSimple_stmt`:L3047 · `VisitSuite`:L3081
`StmtGuaranteesExit`:L3122 · `VisitArith_expr`:L3138 · `VisitExcept_clause`:L3178 · `VisitMatch_stmt`:L3199
`VisitMatch_arm`:L3213 · `BindPatternIdentifiers`:L3243 · `VisitRecord_def`:L3249 · `RegisterAdtSumTypeAlias`:L3307
`RegisterAdtVariantSymbol`:L3348 · `FinishGuardSuite`:L3386 · `VisitIfBranchesWithPatterns`:L3404
`BindPatternVarSingle`:L3495 · `BindPatternNode`:L3518 · `BindPatternNodeLegacyUnused`:L3607
`RegisterGenericParams`:L3642 · `DeclareVariableSymbol`:L3654 · `DeclareVariableSymbolFromToken`:L3669
`CreateSymbol`:L3703 · `CreateSymbolFromToken`:L3725 · `RegisterSymbol`:L3775 · `AddSymbolReferenceDirect`:L3802
`AddSemanticTokenDirect`:L3814 · `ProcessFStringNode`:L3835 · `AddSymbolReference`:L3973 · `AddSemanticToken`:L3986
`AddDiagnostic`:L3999 · `AddDiagnosticDirect`:L4022 · `GetSemanticTokenTypeForSymbol`:L4044
`GetTokenCoords`:L4061 · `ExtractDictTypes`:L4070 · `ExtractElementType`:L4098 · `IsSubtypeOf`:L4124
`FindCommonBaseType`:L4137 · `HomogenizeTypes`:L4171 · `TryExtractSimpleIdentifier`:L4244
`FindFirstIdentifier`:L4253 · `Truncate`:L4264 · `EnterScope`:L4267 · `Dispose`:L4313 · `SyntaxError`:L4321

### `Pilot.LanguageServer/Semantics/PilotSymbol.cs` (392 lines)
`GetLspDocumentSymbolKind`:L189 · `FormatHoverMarkdown`:L212 · `Resolve`:L286 · `GetAllVisibleSymbols`:L296
`FindSymbolAtPosition`:L320 · `FindScopeAtLine`:L331 · `FindDeepestScope`:L336 · `EncodeSemanticTokens`:L346

### `Preprocessing/PilotPreprocessor.cs` (983 lines)
`ToString`:L24 · `IsWarningSuppressed`:L69 · `Preprocess`:L103 · `ProcessDirective`:L219 · `ProcessPragma`:L447
`EvaluateCondition`:L490 · `SplitLines`:L518 · `IsValidSymbolName`:L556 · `StripQuotes`:L571
`TokenizeCondition`:L588 · `Parse`:L661 · `ParseOr`:L669 · `ParseAnd`:L681 · `ParseEquality`:L693
`ParseUnary`:L705 · `ParsePrimary`:L715 · `GetIndent`:L767 · `FindTrailingCommentIndex`:L779
`IsBlockStartLine`:L801 · `EndsWithBlockColon`:L831 · `DesugarColonBlocks`:L877

### `Runtime/Builtins/PilotBuiltinRegistry.cs` (809 lines)
`Register`:L25 · `Register`:L30 · `TryGet`:L38 · `GetAll`:L41 · `RegisterDefaults`:L43 · `GetOperatorFunction`:L787

### `Runtime/Bytecode/PilotBytecodeChunk.cs` (586 lines)
`Emit`:L89 · `EmitJump`:L97 · `PatchJump`:L107 · `AddJumpTable`:L121 · `AddTypeDescriptor`:L130
`AddConstant`:L139 · `AddIdentifier`:L155 · `AddNestedChunk`:L171 · `AddAstFallback`:L180
`GetOrAllocateLocalSlot`:L189 · `TryGetLocalSlot`:L203 · `Write`:L210 · `Read`:L304 · `ReadV2Extensions`:L392
`WriteValue`:L453 · `ReadValue`:L522 · `ReadKeywordVariant`:L545 · `ReadList`:L554 · `ReadTuple`:L565
`ReadRange`:L576

### `Runtime/Bytecode/PilVirtualMachine.cs` (1340 lines)
`Execute`:L28 · `DeclareTypeDescriptorInContext`:L998 · `RefreshLayouts`:L1051 · `CheckRuntimeTypeMatch`:L1063
`MatchesClassHierarchy`:L1159 · `EnsureStackCapacity`:L1180 · `InvokeCallable`:L1189 · `InvokeMethod`:L1228
`CreateIterator`:L1258 · `EnumerateRange`:L1295 · `EnumerateString`:L1309 · `EnumerateObjects`:L1317
`EnumerateSingle`:L1325 · `CreateParameters`:L1330

### `Runtime/Execution/PilotEngine.cs` (631 lines)
`Execute`:L21 · `Execute`:L27 · `ExecuteFile`:L153 · `EvaluateExpression`:L168 · `InvokeFunction`:L186
`CreateGenerator`:L322 · `CreateGenerator`:L362 · `RunnerBody`:L367 · `Next`:L422 · `Close`:L452
`TranspileTo`:L467 · `TranspileFrom`:L473 · `Compile`:L479 · `CompileFile`:L489 · `CreateCallScope`:L499
`SnapshotOldCalls`:L562 · `Write`:L608 · `Write`:L614 · `WriteLine`:L620 · `Flush`:L626

### `Runtime/Execution/PilotInterpreterVisitor.cs` (4275 lines)
`VisitProgram`:L84 · `VisitStmt`:L129 · `VisitSimple_stmt`:L143 · `VisitSuite`:L160 · `VisitLet_stmt`:L184
`VisitVar_stmt`:L191 · `VisitConst_stmt`:L198 · `DeclareBinding`:L205 · `DeclareRecordPattern`:L233
`ProcessRecordRestPattern`:L261 · `ProcessRecordFieldPattern`:L293 · `DeclareListPattern`:L355
`DeclareTuplePattern`:L428 · `VisitPattern_assign_stmt`:L501 · `VisitExpr_stmt`:L519 · `TryUnwrapAtomExpr`:L561
`AnalyzeTarget`:L601 · `EvaluateAtomExprPrefix`:L678 · `AssignTarget`:L721 · `VisitFunc_def`:L856
`ContainsYield`:L940 · `ContainsYieldCore`:L946 · `VisitClass_def`:L958 · `VisitState_def`:L1123
`VisitMacro_def`:L1176 · `VisitType_alias_stmt`:L1204 · `VisitEnum_def`:L1267 · `VisitRecord_def`:L1286
`VisitStruct_def`:L1325 · `DefaultValueForType`:L1385 · `RegisterAdtDefinition`:L1394
`RefreshAllAdtAndRecordLayouts`:L1417 · `IsKnownTypeOrBinding`:L1429 · `ExtractGenericParamNames`:L1445
`ExtractAdtFields`:L1451 · `VisitExtension_def`:L1480 · `VisitAdapter_def`:L1519 · `VisitSteal_stmt`:L1548
`VisitDelegate_stmt`:L1588 · `VisitContract_forward_stmt`:L1609 · `VisitIf_stmt`:L1634 · `FindBranchPattern`:L1681
`FindBranchPatternGuard`:L1700 · `IsBareOptionalBindingPattern`:L1719 · `VisitWhile_stmt`:L1744
`VisitLabeled_stmt`:L1828 · `VisitBlock`:L1854 · `VisitFor_stmt`:L1869 · `TryGetSimpleForTargetName`:L2081
`VisitWith_stmt`:L2099 · `AssignForTarget`:L2179 · `VisitMatch_stmt`:L2205 · `VisitGuard_stmt`:L2236
`VisitDefer_stmt`:L2287 · `IsKnownExceptionTypeName`:L2344 · `VisitTry_stmt`:L2354 · `IsExceptionMatching`:L2471
`VisitAssert_stmt`:L2495 · `VisitFlow_stmt`:L2509 · `VisitTestlist`:L2567 · `VisitTest`:L2584
`VisitPipe_expr`:L2609 · `VisitSet_query_expr`:L2626 · `VisitCoalesce_expr`:L2635 · `VisitOr_test`:L2655
`VisitAnd_test`:L2669 · `VisitNot_test`:L2683 · `VisitComparison`:L2692 · `VisitRange_expr`:L2735
`VisitBitor_expr`:L2750 · `VisitBitxor_expr`:L2764 · `VisitBitand_expr`:L2778 · `VisitShift_expr`:L2792
`VisitArith_expr`:L2809 · `VisitTerm`:L2826 · `EvaluateProjection`:L2878 · `VisitFactor`:L2938
`VisitUnary_op`:L2947 · `VisitPower`:L2986 · `VisitAtom_expr`:L3001 · `TryInvokeDirectMemberCall`:L3089
`EvaluateCascade`:L3287 · `DispatchCall`:L3336 · `InvokeCallableSingle`:L3455 · `ExtractCollectionItems`:L3560
`EvaluateCallArguments`:L3615 · `EvaluateArglist`:L3621 · `BindArgumentsToFunction`:L3626
`IsCollectionOrTupleParameterType`:L3706 · `VisitAtom`:L3736 · `VisitSymbol_literal`:L3754
`ResolveIdentifier`:L3767 · `VisitLiteral`:L3787 · `IsSimpleIdentifier`:L3875 · `ParseFStringSegments`:L3888
`EvaluateFString`:L3934 · `UnescapeString`:L3978 · `VisitCollection_expr`:L3991 · `VisitDictorsetmaker`:L4022
`VisitTestlist_comp`:L4105 · `VisitInline_foreign_block`:L4121 · `GetBlockBodyText`:L4242

### `Runtime/Execution/PilotPatternMatcher.cs` (882 lines)
`Match`:L25 · `MatchPatternInternal`:L46 · `MatchOrPattern`:L57 · `MatchAndPattern`:L86 · `MatchNotPattern`:L103
`MatchPrimaryPattern`:L134 · `MatchRangePattern`:L249 · `EvaluateRangeBound`:L295 · `ParseSignedNumber`:L315
`ParseUnsignedIntToken`:L326 · `MatchKeywordPattern`:L335 · `MatchTypeOrVarPattern`:L396 · `MatchAdtFields`:L628
`MatchRecordFields`:L676 · `MatchPropertyPattern`:L715 · `MatchesObjectType`:L759 · `MatchesPrimitiveType`:L787
`MatchCollectionSlice`:L810

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

### `Runtime/Packaging/PilotPackageManager.cs` (376 lines)
`RegisterAdapter`:L34 · `TryGetAdapter`:L42 · `Init`:L50 · `Add`:L85 · `Remove`:L123 · `Restore`:L159 · `List`:L246
`Pack`:L296 · `ParsePackageSpec`:L338 · `ComputeHash`:L361 · `GetEffectiveModulesDirectory`:L367

### `Runtime/Values/PilotAdt.cs` (868 lines)
`PilotFieldMemoryLayout`:L35 · `PilotVariantMemoryLayout`:L46 · `ComputeForAdt`:L90 · `ComputeForRecord`:L155
`FormsDirectValueCycle`:L201 · `EstimateInlineTypeSizeAndAlign`:L274 · `StripGenericsAndModifiers`:L343
`SplitTopLevel`:L358 · `AlignUp`:L382 · `GetFieldIndex`:L456 · `Instantiate`:L465 · `CoerceFieldValue`:L537
`ToString`:L546 · `AddVariant`:L569 · `TryGetVariant`:L578 · `TryGetVariantByTag`:L581 · `ToString`:L592
`Equals`:L610 · `Equals`:L627 · `GetHashCode`:L629 · `ToString`:L641 · `IsFloatTypeName`:L704
`GetFieldIndex`:L707 · `Instantiate`:L710 · `ToString`:L776 · `Equals`:L793 · `Equals`:L805 · `GetHashCode`:L807
`ToString`:L818 · `Equals`:L844 · `Equals`:L856 · `GetHashCode`:L858 · `ToString`:L866

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

### `Runtime/Values/PilotValue.cs` (1218 lines)
`FromBool`:L37 · `FromInt`:L38 · `FromUInt`:L39 · `FromFloat`:L40 · `FromDecimal`:L41 · `FromChar`:L42
`FromString`:L43 · `FromKeyword`:L44 · `FromKeyword`:L45 · `FromList`:L46 · `FromTuple`:L47 · `FromMap`:L48
`FromSet`:L49 · `FromRange`:L50 · `FromMatrix`:L51 · `FromFunction`:L52 · `FromObject`:L53 · `FromGenerator`:L54
`FromLazy`:L55 · `FromFuture`:L56 · `FromForeign`:L57 · `FromMethodGroup`:L58 · `FromNativeArray`:L59
`FromNativeMap`:L60 · `FromAdtType`:L61 · `FromAdtUnit`:L62 · `FromAdtBoxed`:L63 · `FromRecordUnboxedEmpty`:L64
`FromRecordUnboxedPairF64`:L65 · `FromRecordBoxed`:L67 · `FromKeywordVariant`:L68 · `TryCreateUnboxedAdtSingle`:L70
`PilotValue`:L93 · `PilotValue`:L94 · `PilotValue`:L95 · `PilotValue`:L96 · `PilotValue`:L97 · `PilotValue`:L98
`PilotValue`:L99 · `PilotValue`:L100 · `PilotValue`:L101 · `AsInt`:L129 · `AsFloat`:L141 · `AsString`:L153
`AsDecimal`:L154 · `AsChar`:L155 · `AsKeyword`:L156 · `AsList`:L158 · `AsTuple`:L159 · `AsMap`:L160 · `AsSet`:L161
`AsRange`:L162 · `AsMatrix`:L163 · `AsFunction`:L164 · `AsObject`:L165 · `AsGenerator`:L166 · `AsLazy`:L167
`AsFuture`:L168 · `AsMethodGroup`:L169 · `AsNativeArray`:L170 · `AsNativeMap`:L171 · `AsForeign`:L172
`AsAdtType`:L178 · `AsKeywordVariant`:L179 · `GetAdtVariantDefinition`:L181 · `GetAdtTag`:L189
`GetAdtFieldCount`:L191 · `GetAdtField`:L193 · `GetRecordDefinition`:L216 · `GetRecordFieldCount`:L223
`GetRecordField`:L225 · `CreateClassConstructor`:L245 · `GetMember`:L564 · `EnumerateCollection`:L911
`CreateAppendFunction`:L919 · `ResolveExtensionOrRoot`:L965 · `ResolveRootMember`:L999 · `CloneValue`:L1020
`Equals`:L1036 · `EqualsAdt`:L1071 · `EqualsRecord`:L1086 · `ComputeAdtHashCode`:L1101
`ComputeRecordHashCode`:L1111 · `FormatAdt`:L1120 · `FormatRecord`:L1130 · `EqualsList`:L1144 · `EqualsTuple`:L1151
`EqualsMap`:L1158 · `Equals`:L1168 · `GetHashCode`:L1170 · `ToString`:L1183

### `Runtime/Venv/PilotVirtualEnvironment.cs` (416 lines)
`Load`:L32 · `Save`:L79 · `Create`:L139 · `Detect`:L202 · `Activate`:L234 · `Deactivate`:L262
`GenerateActivationScripts`:L269 · `GeneratePowerShellScript`:L276 · `GenerateCmdScripts`:L318
`GeneratePosixScript`:L351 · `GenerateLauncher`:L392

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

### `vscode-pilot/src/extension.ts` (619 lines)
`activate`:L248 · `addOrUpdateLaunchJson`:L541 · `deactivate`:L619

---

## 4. Complete Codebase File & Type Symbol Inventory

### .agents
- `.agents/hooks.json` (23 L)
- `.agents/scripts/sync_ai_index_hook.ps1` (41 L)
- `.agents/scripts/update_ai_index.ps1` (311 L)

### .jules
- `.jules/bolt.md` (8 L)

### (Repository Root)
- `.gitignore` (74 L)
- `AGENTS.md` (51 L)
- `fix_type.sh` (4 L)
- `input.txt` (187 L)
- `MILESTONES.md` (715 L)
- `Pilot.csproj` (42 L)
- `Pilot.slnx` (5 L)
- `README.md` (756 L)

### Analysis
- `Analysis/ContractVerification/PilotContractVerifier.cs` (227 L) — class `PilotContractVerifier`:L14, class `FunctionContractInfo`:L21
- `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs` (344 L) — class `UnitDimensionSignature`:L13, class `PilotDimensionChecker`:L101
- `Analysis/Ownership/PilotBorrowChecker.cs` (503 L) — enum `ResourceState`:L13, enum `ResourceAffinity`:L25, class `ResourceBinding`:L44, class `PilotBorrowChecker`:L68
- `Analysis/PatternMatching/PilotExhaustivenessChecker.cs` (1800 L) — class `PilotExhaustivenessChecker`:L21, record `MatchPat`:L458, record `WildPat`:L459, record `OrPat`:L463, record `CtorPat`:L464, enum `CtorFamily`:L466, record `CtorDesc`:L483
- `Analysis/PilotAnalysisPipeline.cs` (88 L) — class `PilotStaticAnalysisResult`:L16, class `PilotAnalysisPipeline`:L39
- `Analysis/TypeSystem/PilotType.cs` (259 L) — enum `PilotTypeKind`:L10, class `PilotType`:L32
- `Analysis/TypeSystem/PilotTypeInferenceEngine.cs` (522 L) — class `PilotAnalysisDiagnostic`:L12, class `PilotTypeInferenceEngine`:L38

### antlr
- `antlr/concurrency/ConcurrencyLexer.g4` (23 L)
- `antlr/concurrency/ConcurrencyParser.g4` (51 L)
- `antlr/core/CoreLexer.g4` (222 L)
- `antlr/core/CoreParser.g4` (730 L)
- `antlr/csharp/CSharpLexer.g4` (13 L)
- `antlr/csharp/CSharpParser.g4` (54 L)
- `antlr/database/DatabaseLexer.g4` (68 L)
- `antlr/database/DatabaseParser.g4` (163 L)
- `antlr/Pilot.g4` (1653 L)
- `antlr/realtime/RealtimeLexer.g4` (34 L)
- `antlr/realtime/RealtimeParser.g4` (32 L)
- `antlr/udsa/UDSALexer.g4` (19 L)
- `antlr/udsa/UDSAParser.g4` (37 L)

### Cli
- `Cli/PilotRepl.cs` (290 L) — class `PilotRepl`:L15
- `Cli/Program.cs` (1172 L) — class `Program`:L18

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
- `Compiler/PilotBytecodeCompiler.cs` (2162 L) — class `PilotBytecodeCompiler`:L17, class `LoopContext`:L225
- `Compiler/PilotCompilationException.cs` (19 L) — class `PilotCompilationException`:L8
- `Compiler/PilotCompilationResult.cs` (42 L) — class `PilotCompilationResult`:L9
- `Compiler/PilotCompiledProgram.cs` (559 L) — class `PilotCompiledProgram`:L20
- `Compiler/PilotCompiler.cs` (197 L) — class `PilotCompiler`:L17
- `Compiler/PilotCompilerOptions.cs` (110 L) — enum `CompilationTarget`:L9, class `PilotCompilerOptions`:L31
- `Compiler/PilotMatchBytecodeLowerer.cs` (2009 L) — class `PilotMatchBytecodeLowerer`:L18, class `StagedBinding`:L29, enum `ArmBranchKind`:L43, class `AnalyzedBranch`:L52
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
- `examples/showcase/run_all.ps1` (14 L)

### Native
- `Native/Assembly/PilotAssemblyEmitter.cs` (213 L) — class `PilotAssemblyEmitter`:L13
- `Native/CAbi/PilotCAbiEmitter.cs` (473 L) — class `PilotCAbiEmitter`:L19, class `StructDescriptor`:L185, class `AdtDescriptor`:L191, class `FunctionDescriptor`:L197, class `CAbiExtractionVisitor`:L204
- `Native/Llvm/PilotLlvmEmitter.cs` (1869 L) — class `PilotLlvmEmitter`:L21
- `Native/PilotNativeCompiler.cs` (373 L) — enum `NativeCompilationTarget`:L16, class `PilotNativeOptions`:L31, class `PilotNativeResult`:L46, class `PilotNativeCompiler`:L90
- `Native/Runtime/pilot_runtime.h` (147 L)
- `Native/Wasm/PilotWasmEmitter.cs` (878 L) — class `PilotWasmEmitter`:L18

### Pilot.LanguageServer
- `Pilot.LanguageServer/Lsp/PilotLspServer.cs` (743 L) — class `PilotLspServer`:L16
- `Pilot.LanguageServer/Pilot.LanguageServer.csproj` (15 L)
- `Pilot.LanguageServer/Program.cs` (46 L) — class `Program`:L9
- `Pilot.LanguageServer/Semantics/PilotDiagnosticCatalog.cs` (255 L) — class `PilotDiagnosticDescriptor`:L9, class `PilotDiagnosticCatalog`:L54
- `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (4373 L) — class `PilotSemanticAnalyzer`:L10, class `ScopeRestorer`:L4304, class `LspErrorListener`:L4316
- `Pilot.LanguageServer/Semantics/PilotSymbol.cs` (392 L) — enum `PilotSymbolKind`:L7, enum `PilotDiagnosticSeverity`:L35, class `PilotCodeAction`:L43, class `PilotDiagnostic`:L55, class `PilotSemanticTokenLegend`:L68, class `PilotSemanticToken`:L142, class `PilotSymbol`:L151, class `PilotSymbolReference`:L256, class `PilotScope`:L264, class `PilotAnalysisResult`:L310

### Pilot.Tests
- `Pilot.Tests/LspMessageHelper.cs` (13 L) — class `LspMessageHelper`:L6
- `Pilot.Tests/Pilot.Tests.csproj` (26 L)
- `Pilot.Tests/PilotAdtAndExhaustivenessTests.cs` (689 L) — class `PilotAdtAndExhaustivenessTests`:L33
- `Pilot.Tests/PilotAdvancedFeaturesTests.cs` (263 L) — class `PilotAdvancedFeaturesTests`:L11
- `Pilot.Tests/PilotBurstAndNativeTypesTests.cs` (150 L) — class `PilotBurstAndNativeTypesTests`:L14
- `Pilot.Tests/PilotCompilationAndScriptingTests.cs` (174 L) — class `PilotCompilationAndScriptingTests`:L11
- `Pilot.Tests/PilotComprehensiveKeywordValidationTests.cs` (452 L) — class `PilotComprehensiveKeywordValidationTests`:L13
- `Pilot.Tests/PilotContextContractsPatternsTests.cs` (297 L) — class `PilotContextContractsPatternsTests`:L11
- `Pilot.Tests/PilotContractDelegationAndStealingTests.cs` (234 L) — class `PilotContractDelegationAndStealingTests`:L10
- `Pilot.Tests/PilotDebuggingTests.cs` (265 L) — class `PilotDebuggingTests`:L12
- `Pilot.Tests/PilotExceptionProtectionsTests.cs` (196 L) — class `PilotExceptionProtectionsTests`:L11
- `Pilot.Tests/PilotExecutionEngineTests.cs` (260 L) — class `PilotExecutionEngineTests`:L9
- `Pilot.Tests/PilotFeedbackFeaturesTests.cs` (298 L) — class `PilotFeedbackFeaturesTests`:L11
- `Pilot.Tests/PilotHotReloadTests.cs` (159 L) — class `PilotHotReloadTests`:L11
- `Pilot.Tests/PilotIdeDiagnosticsTests.cs` (363 L) — class `PilotIdeDiagnosticsTests`:L10
- `Pilot.Tests/PilotLanguageTests.cs` (1541 L) — class `ParserHelper`:L9, class `ErrorTracker`:L36, class `TypeSafetyTests`:L52, class `DatabaseSetOperationsTests`:L125, class `ConcurrencyTests`:L178, class `RealTimeTests`:L256, class `UdsaMetaprogrammingTests`:L335, class `GuardDeferTests`:L398, class `PostgresDatabaseTests`:L465, class `CSharpRobustnessTests`:L571, class `ExtendedLanguageFeaturesTests`:L694, class `PublicClass`:L705, class `ProtectedClass`:L710, struct `InternalPoint`:L711, interface `IPublicService`:L716, enum `Status`:L720, class `MathUtils`:L947, class `PatternMatchingAndCollectionsTests`:L975, class `BuiltInTypesAndMatrixTests`:L1081, class `PreprocessingTests`:L1176, class `FileSuiteTests`:L1314, class `PolyglotInteropLanguageTests`:L1357, class `RootObjectAndLispMetaprogrammingTests`:L1451
- `Pilot.Tests/PilotMethodGroupTests.cs` (248 L) — class `PilotMethodGroupTests`:L11
- `Pilot.Tests/PilotNativeCompilerTests.cs` (290 L) — class `PilotNativeCompilerTests`:L16
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
- `Pilot.Tests/PilotVirtualEnvironmentTests.cs` (118 L) — class `PilotVirtualEnvironmentTests`:L10
- `Pilot.Tests/PilotVirtualMachineTests.cs` (340 L) — class `PilotVirtualMachineTests`:L17

### Preprocessing
- `Preprocessing/PilotPreprocessor.cs` (983 L) — enum `PreprocessorDiagnosticSeverity`:L9, class `PreprocessorDiagnostic`:L16, class `CodeRegion`:L28, class `WarningSuppression`:L35, class `PilotPreprocessorOptions`:L42, class `PilotPreprocessResult`:L60, class `PilotPreprocessor`:L91, class `IfFrame`:L579, class `ConditionParser`:L649, class `ColonBlockFrame`:L755

### Runtime
- `Runtime/Adapters/PilotTypeAdapterRegistry.cs` (210 L) — class `PilotTypeAdapterRegistry`:L14, class `AdapterKeyComparer`:L195
- `Runtime/Builtins/IBuiltinFunction.cs` (15 L) — interface `IBuiltinFunction`:L10
- `Runtime/Builtins/PilotBuiltinRegistry.cs` (809 L) — class `PilotBuiltinRegistry`:L15
- `Runtime/Burst/PilotBurstEngine.cs` (145 L) — class `PilotBurstEngine`:L19
- `Runtime/Bytecode/PilotBytecodeChunk.cs` (586 L) — class `PilotJumpTable`:L12, class `PilotSerializedFieldDescriptor`:L29, class `PilotSerializedVariantDescriptor`:L38, class `PilotBytecodeTypeDescriptor`:L48, class `PilotBytecodeChunk`:L62
- `Runtime/Bytecode/PilotInstruction.cs` (46 L) — struct `PilotInstruction`:L11
- `Runtime/Bytecode/PilotOpcode.cs` (225 L) — enum `PilotOpcode`:L11
- `Runtime/Bytecode/PilVirtualMachine.cs` (1340 L) — class `PilVirtualMachine`:L20
- `Runtime/Concurrency/PilotFuture.cs` (72 L) — class `PilotFuture`:L11
- `Runtime/Contracts/PilotContractRegistry.cs` (313 L) — class `PilotContractRegistry`:L14
- `Runtime/ControlFlow/PilotControlFlowSignals.cs` (99 L) — enum `PilotFlowSignalKind`:L9, class `PilotControlFlowSignal`:L21, class `PilotReturnSignal`:L29, class `PilotBreakSignal`:L42, class `PilotContinueSignal`:L55, class `PilotYieldSignal`:L68, class `PilotRuntimeException`:L81
- `Runtime/ControlFlow/PilotExceptions.cs` (123 L) — class `PilotTypeException`:L9, class `PilotDivideByZeroException`:L18, class `PilotIndexOutOfRangeException`:L27, class `PilotKeyNotFoundException`:L36, class `PilotNullReferenceException`:L45, class `PilotAssertionException`:L54, class `PilotRecursionException`:L63, class `PilotContractException`:L72, class `PilotTimeoutException`:L81, class `PilotPatternMatchingException`:L90, class `PilotContextException`:L99, class `PilotUserException`:L108
- `Runtime/Environment/PilotBinding.cs` (22 L) — class `PilotBinding`:L8
- `Runtime/Environment/PilotEnvironment.cs` (59 L) — class `PilotEnvironment`:L9
- `Runtime/Environment/PilotScope.cs` (137 L) — class `PilotScope`:L10
- `Runtime/Execution/IPilotEngine.cs` (56 L) — interface `IPilotEngine`:L10
- `Runtime/Execution/PilotEngine.cs` (631 L) — class `PilotEngine`:L19, class `GeneratorRunner`:L332, class `MergedTextWriter`:L595
- `Runtime/Execution/PilotExecutionContext.cs` (135 L) — class `PilotExecutionContext`:L13
- `Runtime/Execution/PilotExecutionOptions.cs` (87 L) — class `PilotExecutionOptions`:L13
- `Runtime/Execution/PilotExecutionResult.cs` (56 L) — class `PilotExecutionResult`:L10
- `Runtime/Execution/PilotInterpreterVisitor.cs` (4275 L) — class `PilotInterpreterVisitor`:L19, struct `FStringSegment`:L41, enum `TargetAssignmentKind`:L57, class `TargetAssignmentInfo`:L66
- `Runtime/Execution/PilotPatternMatcher.cs` (882 L) — class `PilotPatternMatcher`:L16
- `Runtime/Extensions/PilotLexicalExtension.cs` (54 L) — class `PilotLexicalExtension`:L11
- `Runtime/Generative/PilotClassNode.cs` (127 L) — class `PilotFieldNode`:L11, class `PilotClassNode`:L38
- `Runtime/HotReload/PilotHotReloader.cs` (732 L) — class `PilotSymbolSnapshot`:L19, class `PilotPatchRecord`:L165, class `PilotPatchResult`:L208, class `PilotHotReloadSession`:L242
- `Runtime/Interop/IPolyglotProvider.cs` (31 L) — interface `IPolyglotProvider`:L10
- `Runtime/Interop/IPolyglotTypeConverter.cs` (20 L) — interface `IPolyglotTypeConverter`:L9
- `Runtime/Interop/PolyglotRegistry.cs` (670 L) — class `PolyglotRegistry`:L12, class `DefaultPythonProvider`:L72, class `DefaultCSharpProvider`:L149, class `DefaultRustProvider`:L221, class `DefaultJavaScriptProvider`:L264, class `DefaultJavaProvider`:L318, class `DefaultAsmProvider`:L370, class `DefaultCProvider`:L439, class `DefaultCppProvider`:L484, class `DefaultLuaProvider`:L528, class `DefaultPostgreSqlProvider`:L578, class `DefaultMySqlProvider`:L652
- `Runtime/Interop/PolyglotTypeBridge.cs` (345 L) — class `PolyglotTypeBridge`:L16
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
- `Runtime/Values/PilotAdt.cs` (868 L) — class `PilotAdtFieldDefinition`:L14, record `PilotFieldMemoryLayout`:L35, record `PilotVariantMemoryLayout`:L46, class `PilotAdtMemoryLayout`:L58, class `PilotAdtFloatSlot`:L394, class `PilotAdtVariantDefinition`:L403, class `PilotAdtDefinition`:L552, class `PilotAdtInstance`:L599, class `PilotRecordDefinition`:L659, class `PilotRecordInstance`:L782, class `PilotKeywordVariant`:L833
- `Runtime/Values/PilotFunction.cs` (258 L) — class `PilotParameter`:L12, class `PilotFunction`:L37
- `Runtime/Values/PilotGenerator.cs` (123 L) — enum `PilotGeneratorState`:L13, class `PilotGenerator`:L25
- `Runtime/Values/PilotKeyword.cs` (96 L) — class `PilotKeyword`:L12
- `Runtime/Values/PilotLazy.cs` (55 L) — class `PilotLazy`:L9
- `Runtime/Values/PilotMatrix.cs` (209 L) — class `PilotMatrix`:L9
- `Runtime/Values/PilotMethodGroup.cs` (551 L) — class `PilotMethodGroup`:L20
- `Runtime/Values/PilotObject.cs` (649 L) — class `PilotClassDefinition`:L11, class `PilotObject`:L393
- `Runtime/Values/PilotRange.cs` (98 L) — class `PilotRange`:L10
- `Runtime/Values/PilotValue.cs` (1218 L) — struct `PilotValue`:L15
- `Runtime/Values/PilotValueKind.cs` (37 L) — enum `PilotValueKind`:L6
- `Runtime/Venv/PilotVirtualEnvironment.cs` (416 L) — class `PilotVenvOptions`:L11, class `PilotVenvConfig`:L22, class `PilotVirtualEnvironment`:L104

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
- `vscode-pilot/src/extension.ts` (619 L)
- `vscode-pilot/syntaxes/pilot.tmLanguage.json` (369 L)
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
