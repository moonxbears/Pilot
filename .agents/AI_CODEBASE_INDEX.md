# Pilot — AI-Only Codebase Navigation & Symbol Index

> **STRICTLY FOR AI AGENTS**: Do not link this file in user-facing documentation. Use exact paths and `:L<line>` anchors below with `read_file` (`StartLine`/`EndLine`) to jump directly to code without exploratory directory/grep bouncing.
> **Auto-Sync Stamp**: `49628F1CDE56324A2947FD0EF41F0DD64E4A2944DDCFE4687128DD347EC82BB5` | **Total Indexed Files**: 322 | **Regenerate**: `pwsh -NoProfile -File .agents/scripts/update_ai_index.ps1`

<!-- BEGIN MANUAL NOTES -->
## 1. Instant Task-to-File Routing Table (Consult First — Zero Blind Searching)

| Subsystem / Task | Primary Entry Files | Key Notes |
| :--- | :--- | :--- |
| **Roadmap, Concurrency & Invariants** | `MILESTONES.md` | Single source of truth for M1–M48 status, architecture invariants. |
| **Master Lexer & Tokenizer** | `crates/pilot-lexer/src/lib.rs` | Zero-allocation zero-copy token stream, logos/regex-based lexer. |
| **CST & Concrete Syntax Nodes** | `crates/pilot-syntax/src/lib.rs` | Red-green syntax tree nodes, `SyntaxKind`, lossless trivia. |
| **Parser & AST Arena** | `crates/pilot-parser/src/lib.rs`, `crates/pilot-ast/src/lib.rs` | Recursive descent Pratt parser, 16-byte cache-line packed `AstNode` arena. |
| **Semantic Analysis & Type Inference** | `crates/pilot-analysis/src/pipeline.rs`, `infer.rs`, `patterns.rs` | Bidirectional HM type inference, pattern exhaustiveness, typestate verification. |
| **Perceus Linear Memory Management** | `crates/pilot-perceus/src/lib.rs` | Zero-GC Perceus reference counting, in-place reuse, uniqueness typing. |
| **PilVM Virtual Machine & Bytecode** | `crates/pilot-vm/src/lib.rs`, `crates/pilot-bytecode/src/lib.rs` | High-throughput bytecode VM, chunk compiler, frame stack execution. |
| **Native AOT & GPU Code Generation** | `crates/pilot-codegen/src/lib.rs`, `c_target.rs`, `llvm.rs`, `wasm.rs`, `gpu.rs` | Native C11, LLVM IR, WASM, and GPU (CUDA, ROCm, Vulkan, WGSL) code generators. |
| **Polyglot Interoperability Engine** | `crates/pilot-polyglot/src/lib.rs`, `bridge.rs`, `marshal.rs` | Inline Python, JS, Lua, Java, C, C++, ASM execution with cross-language marshalling. |
| **Incremental Computation & Salsa DB** | `crates/pilot-db/src/lib.rs` | Salsa-based incremental computation database, AST/Type queries. |
| **Language Server Protocol (LSP)** | `crates/pilot-lsp/src/lib.rs`, `server.rs` | Native LSP server (30+ capabilities, 23-type semantic tokens, signatures, hover). |
| **Standard Library & Builtin Intrinsics** | `crates/pilot-stdlib/src/lib.rs`, `crates/pilot-runtime/src/lib.rs` | Core standard library, system calls, collections, I/O primitives. |
| **Package Management & Virtual Envs** | `crates/pilot-pkg/src/lib.rs`, `crates/pilot-venv/src/lib.rs` | `.pilpkg` package resolution, lockfiles, virtual environments. |
| **CLI Driver & Interactive REPL** | `crates/pilot-cli/src/main.rs`, `crates/pilot-repl/src/lib.rs` | 14 CLI commands (`run`, `build`, `check`, `fmt`, `test`, `repl`, `lsp`, etc.). |
| **VS Code Extension** | `vscode-pilot/src/extension.ts`, `vscode-pilot/syntaxes/pilot.tmLanguage.json` | Native LSP client launcher, TextMate grammar, debugger integration. |
| **Integration & Differential Test Suites** | `tests/*.pilot`, `tests/differential/*.pilot`, `crates/pilot-cli/src/main.rs` | `cargo test --all`, `pilot test`, `pilot test --differential` (PilVM, Native, LLVM, WASM). |

### Cross-Layer Checklist When Adding/Modifying a Language Feature
1. **Lexer / Syntax**: `crates/pilot-syntax/src/lib.rs` (`SyntaxKind`), `crates/pilot-lexer/src/lib.rs` (`TokenKind`).
2. **Parser & AST**: `crates/pilot-parser/src/lib.rs`, `crates/pilot-ast/src/lib.rs` (`NodeKind`, 16-byte `AstNode` arena).
3. **Static Analysis & Type System**: `crates/pilot-analysis/src/infer.rs`, `patterns.rs`, `pipeline.rs`.
4. **Perceus Memory Lowering**: `crates/pilot-perceus/src/lib.rs`.
5. **Execution & Codegen**: `crates/pilot-vm/src/lib.rs` (PilVM bytecode execution) & `crates/pilot-codegen/src/` (`c_target.rs`, `llvm.rs`, `wasm.rs`, `gpu.rs`).
6. **LSP & Tooling**: `crates/pilot-lsp/src/server.rs` (semantic tokens, hover, diagnostics) + `vscode-pilot/syntaxes/pilot.tmLanguage.json`.
7. **Verification**: Run `cargo test --all`, `cargo run --bin pilot -- test`, and `cargo run --bin pilot -- test --differential`.
<!-- END MANUAL NOTES -->

---

## 2. Master Grammar Map (`antlr/Pilot.g4` Exact Line Anchors)

Total lines in `antlr/Pilot.g4`: **1723**

### Parser Rules (239)
`program`:L15 · `identifier`:L23 · `stmt`:L53 · `simple_stmt`:L58 · `small_stmt`:L62 · `compound_stmt`:L90
`suite`:L142 · `guard_stmt`:L152 · `pattern_guard`:L156 · `defer_stmt`:L161 · `type_annotation`:L169
`return_type`:L173 · `type_spec`:L177 · `error_union_type`:L181 · `error_primary_type`:L185 · `union_type`:L192
`intersection_type`:L196 · `primary_type`:L200 · `base_type`:L204 · `type_arguments`:L215 · `generic_params`:L219
`generic_param`:L223 · `where_type_clause`:L228 · `where_type_item`:L232 · `access_modifier`:L237
`declaration_modifier`:L244 · `binding_pattern`:L254 · `tuple_pattern`:L262 · `list_pattern`:L266
`record_pattern`:L270 · `binding_pattern_item`:L274 · `record_pattern_field`:L278 · `pattern_assign_stmt`:L283
`let_stmt`:L287 · `var_stmt`:L291 · `const_stmt`:L295 · `ref_stmt`:L299 · `dynamic_stmt`:L303
`type_alias_stmt`:L307 · `adt_sum_body`:L312 · `adt_variant`:L318 · `adt_variant_payload`:L322
`adt_field_list`:L326 · `adt_field`:L330 · `annassign_stmt`:L335 · `expr_stmt`:L339 · `channel_send_stmt`:L343
`augassign`:L347 · `func_def`:L367 · `contract_clause`:L371 · `parameters`:L376 · `parameter`:L380
`class_def`:L384 · `state_def`:L389 · `class_bases`:L393 · `struct_def`:L398 · `record_def`:L403
`struct_member`:L408 · `interface_def`:L413 · `enum_def`:L418 · `enum_suite`:L422 · `enum_member`:L426
`actor_def`:L431 · `model_def`:L436 · `property_def`:L445 · `property_body`:L449 · `property_accessor`:L453
`get_accessor`:L458 · `set_accessor`:L462 · `operator_def`:L467 · `overloadable_op`:L471 · `extension_def`:L479
`adapter_def`:L485 · `macro_def`:L490 · `comptime_stmt`:L494 · `comptime_expr`:L498 · `quote_stmt`:L502
`quote_expr`:L506 · `quasiquote_expr`:L510 · `splice_expr`:L514 · `extern_def`:L521 · `extern_member`:L525
`foreign_language`:L531 · `inline_foreign_block`:L535 · `decorators`:L540 · `decorator`:L544 · `labeled_stmt`:L552
`block`:L556 · `if_stmt`:L560 · `while_stmt`:L564 · `for_stmt`:L568 · `for_target`:L572 · `match_stmt`:L577
`match_suite`:L582 · `match_arm`:L586 · `pattern`:L590 · `or_pattern`:L594 · `and_pattern`:L598
`not_pattern`:L602 · `primary_pattern`:L608 · `range_pattern`:L621 · `range_bound`:L625
`signed_number_pattern`:L630 · `keyword_pattern`:L634 · `pattern_arg`:L638 · `relational_pattern`:L643
`collection_pattern`:L647 · `slice_pattern_element`:L651 · `property_pattern`:L656 · `property_pattern_member`:L660
`tuple_pattern_match`:L664 · `type_or_var_pattern`:L668 · `generic_pattern_suffix`:L673 · `try_stmt`:L677
`except_clause`:L681 · `with_stmt`:L685 · `with_item`:L689 · `with_context_binding`:L696 · `flow_stmt`:L700
`pass_stmt`:L708 · `assert_stmt`:L712 · `steal_stmt`:L716 · `delegate_stmt`:L720 · `contract_forward_stmt`:L724
`global_stmt`:L728 · `import_stmt`:L732 · `foreign_import_stmt`:L738 · `foreign_target`:L743 · `import_name`:L747
`import_from`:L751 · `import_as_name`:L755 · `dotted_as_name`:L759 · `import_as_names`:L763
`dotted_as_names`:L767 · `dotted_name`:L771 · `deadline_stmt`:L779 · `within_stmt`:L783 · `periodic_stmt`:L787
`noalloc_stmt`:L791 · `region_stmt`:L795 · `duration_term`:L799 · `spawn_stmt`:L808 · `atomic_stmt`:L812
`sync_stmt`:L816 · `isolate_stmt`:L820 · `select_stmt`:L824 · `select_suite`:L828 · `select_arm`:L832
`select_case_target`:L838 · `dsl_stmt`:L848 · `dsl_args`:L852 · `unit_stmt`:L856 · `unit_expr`:L860
`unit_term`:L864 · `database_stmt`:L875 · `database_def`:L879 · `transaction_stmt`:L884 · `savepoint_stmt`:L888
`savepoint_block`:L892 · `commit_stmt`:L896 · `rollback_stmt`:L900 · `insert_stmt`:L905
`insert_values_group`:L912 · `on_conflict_clause`:L916 · `update_stmt`:L921 · `set_assignment_list`:L928
`set_assignment`:L932 · `delete_stmt`:L937 · `returning_clause`:L943 · `index_clause`:L948 · `index_field`:L952
`index_type`:L956 · `index_hint`:L960 · `query_expr`:L965 · `from_clause`:L969 · `join_clause`:L973
`where_clause`:L977 · `group_clause`:L981 · `order_clause`:L985 · `ordering_term`:L989 · `select_clause`:L993
`select_projection`:L997 · `select_item`:L1002 · `limit_clause`:L1006 · `sql_query_expr`:L1011
`from_source`:L1015 · `lookup_expr`:L1020 · `set_op`:L1025 · `testlist`:L1036 · `exprlist`:L1040 · `test`:L1044
`lambda_expr`:L1049 · `lambda_body`:L1058 · `capture_list`:L1063 · `capture_item`:L1067 · `lambda_params`:L1073
`lambda_param`:L1078 · `pipe_expr`:L1083 · `set_query_expr`:L1088 · `coalesce_expr`:L1095 · `catch_clause`:L1099
`or_test`:L1105 · `and_test`:L1109 · `not_test`:L1113 · `comparison`:L1118 · `comp_op`:L1122 · `range_expr`:L1135
`bitor_expr`:L1139 · `bitxor_expr`:L1143 · `bitand_expr`:L1147 · `shift_expr`:L1151 · `arith_expr`:L1155
`term`:L1159 · `factor`:L1163 · `unary_op`:L1169 · `power`:L1179 · `atom_expr`:L1183 · `trailer`:L1187
`call_quote_op`:L1204 · `cascade_trailer`:L1213 · `cascade_target`:L1217 · `over_clause`:L1227
`subscriptlist`:L1231 · `subscript`:L1235 · `atom`:L1243 · `collection_expr`:L1273 · `collection_element`:L1277
`nameof_expr`:L1281 · `macro_call`:L1285 · `symbol_literal`:L1289 · `literal`:L1295 · `testlist_comp`:L1314
`dictorsetmaker`:L1318 · `dict_entry`:L1323 · `comp_for`:L1328 · `comp_iter`:L1332 · `comp_if`:L1337
`arglist`:L1341 · `argument`:L1345

### Lexer Tokens (263)
`PUBLIC`:L1359 · `PRIVATE`:L1360 · `PROTECTED`:L1361 · `INTERNAL`:L1362 · `STATIC`:L1363 · `PARTIAL`:L1364
`STEAL`:L1365 · `CONTRACTS`:L1366 · `DELEGATE`:L1367 · `PARAMS`:L1368 · `ARGS`:L1369 · `KWARGS`:L1370
`NAMEOF`:L1371 · `EACH`:L1372 · `REPEAT`:L1373 · `ADAPTER`:L1374 · `CONVERTER`:L1375 · `DEF`:L1377 · `FN`:L1378
`CLASS`:L1379 · `STRUCT`:L1380 · `RECORD`:L1381 · `INTERFACE`:L1382 · `PROTOCOL`:L1383 · `ENUM`:L1384
`ACTOR`:L1385 · `LET`:L1386 · `VAR`:L1387 · `CONST`:L1388 · `TYPE`:L1389 · `REF`:L1390 · `DYNAMIC`:L1391
`SYMBOL`:L1392 · `PURE`:L1393 · `COMPTIME`:L1394 · `MUT`:L1395 · `IMMUT`:L1396 · `PROPERTY`:L1397 · `GET`:L1398
`OPERATOR`:L1399 · `EXTENSION`:L1400 · `EXTEND`:L1401 · `STATE`:L1402 · `SELECT`:L1405 · `FROM`:L1406
`WHERE`:L1407 · `JOIN`:L1408 · `INNER`:L1409 · `LEFT`:L1410 · `RIGHT`:L1411 · `FULL`:L1412 · `CROSS`:L1413
`OUTER`:L1414 · `ON`:L1415 · `GROUP`:L1416 · `BY`:L1417 · `HAVING`:L1418 · `ORDER`:L1419 · `ASC`:L1420
`DESC`:L1421 · `ASCENDING`:L1422 · `DESCENDING`:L1423 · `LIMIT`:L1424 · `OFFSET`:L1425 · `INTO`:L1426
`DISTINCT`:L1427 · `UNION`:L1428 · `ALL`:L1429 · `INTERSECT`:L1430 · `EXCEPT`:L1431 · `DIFFERENCE`:L1432
`INDEX`:L1433 · `INDEXED`:L1434 · `LOOKUP`:L1435 · `TABLE`:L1436 · `SCHEMA`:L1437 · `INSERT`:L1438 · `UPDATE`:L1439
`DELETE`:L1440 · `VALUES`:L1441 · `SET_KW`:L1442 · `RETURNING`:L1443 · `CONFLICT`:L1444 · `DO`:L1445
`NOTHING`:L1446 · `OVER`:L1447 · `PARTITION`:L1448 · `WINDOW`:L1449 · `TRANSACTION`:L1450 · `COMMIT`:L1451
`ROLLBACK`:L1452 · `SAVEPOINT`:L1453 · `TO`:L1454 · `CONNECT`:L1455 · `DATABASE`:L1456 · `MODEL`:L1457
`ASYNC`:L1460 · `AWAIT`:L1461 · `SPAWN`:L1462 · `PARALLEL`:L1463 · `CHAN`:L1464 · `CHANNEL`:L1465 · `ATOMIC`:L1466
`SYNC`:L1467 · `LOCK`:L1468 · `ISOLATE`:L1469 · `REALTIME`:L1472 · `DEADLINE`:L1473 · `ON_MISSED`:L1474
`WITHIN`:L1475 · `FALLBACK`:L1476 · `PERIOD`:L1477 · `PERIODIC`:L1478 · `PRIORITY`:L1479 · `JITTER`:L1480
`BOUNDED`:L1481 · `NOALLOC`:L1482 · `STACKALLOC`:L1483 · `REGION`:L1484 · `WCET`:L1485 · `HARD_RT`:L1486
`SOFT_RT`:L1487 · `EXTERN`:L1490 · `CSHARP`:L1491 · `PYTHON`:L1492 · `RUST`:L1493 · `JS`:L1494 · `JAVASCRIPT`:L1495
`JAVA`:L1496 · `ASM`:L1497 · `ASSEMBLY`:L1498 · `CPP`:L1499 · `C_KW`:L1500 · `LUA`:L1501 · `SQL_KW`:L1502
`POSTGRES`:L1503 · `POSTGRESQL`:L1504 · `MYSQL`:L1505 · `MODULE`:L1506 · `LIB`:L1507 · `DSL`:L1510 · `DOMAIN`:L1511
`UNIT`:L1512 · `MACRO`:L1513 · `DEFMACRO`:L1514 · `DEFKEYWORD`:L1515 · `QUOTE`:L1516 · `QUASIQUOTE`:L1517
`SPLICE`:L1518 · `UNQUOTE`:L1519 · `UNQUOTE_SPLICING`:L1520 · `MATCH`:L1521 · `CASE`:L1522 · `WHEN`:L1523
`DEFAULT`:L1524 · `TIMEOUT`:L1525 · `GUARD`:L1528 · `DEFER`:L1529 · `CONTEXT`:L1530 · `REQUIRES`:L1531
`ENSURES`:L1532 · `IF`:L1533 · `ELIF`:L1534 · `ELSE`:L1535 · `THEN`:L1536 · `WHILE`:L1537 · `FOR`:L1538
`IN`:L1539 · `RETURN`:L1540 · `YIELD`:L1541 · `BREAK`:L1542 · `CONTINUE`:L1543 · `PASS`:L1544 · `RAISE`:L1545
`TRY`:L1546 · `CATCH`:L1547 · `FINALLY`:L1548 · `WITH`:L1549 · `AS`:L1550 · `ASSERT`:L1551 · `IMPORT`:L1552
`GLOBAL`:L1553 · `LAMBDA`:L1554 · `AND`:L1555 · `OR`:L1556 · `NOT`:L1557 · `IS`:L1558 · `TRUE`:L1561
`FALSE`:L1562 · `SOME`:L1563 · `NONE`:L1564 · `NULL`:L1565 · `UNDEFINED`:L1566 · `JSON_GET_TEXT`:L1569
`ARROW_LEFT`:L1570 · `ARROW`:L1571 · `FAT_ARROW`:L1572 · `PIPE_FORWARD`:L1573 · `PIPE_BACKWARD`:L1574
`SAFE_CASCADE`:L1575 · `SAFE_NAV`:L1576 · `COALESCE_EQUAL`:L1577 · `COALESCE`:L1578 · `JSON_CONTAINS`:L1579
`JSON_CONTAINED`:L1580 · `ELLIPSIS`:L1581 · `DOTDOT_EQUAL`:L1582 · `DOTDOT_LESS`:L1583 · `DOTDOT`:L1584
`DOUBLESTAR`:L1585 · `DOUBLESLASH`:L1586 · `CALL_QUOTE_DEEP`:L1587 · `CALL_QUOTE_ABI`:L1588
`CALL_QUOTE_REACT`:L1589 · `CALL_QUOTE_ADJOINT`:L1590 · `CALL_QUOTE_EFFECT`:L1591 · `EQEQUAL`:L1592
`NOTEQUAL`:L1593 · `LESSEQUAL`:L1594 · `GREATEREQUAL`:L1595 · `LEFTSHIFT`:L1596 · `RIGHTSHIFT`:L1597
`PLUSEQUAL`:L1598 · `MINEQUAL`:L1599 · `STAREQUAL`:L1600 · `SLASHEQUAL`:L1601 · `PERCENTEQUAL`:L1602
`AMPEREQUAL`:L1603 · `VBAREQUAL`:L1604 · `CIRCUMFLEXEQUAL`:L1605 · `LEFTSHIFTEQUAL`:L1606 · `RIGHTSHIFTEQUAL`:L1607
`DOUBLESTAREQUAL`:L1608 · `DOUBLESLASHEQUAL`:L1609 · `WALRUS`:L1610 · `LPAR`:L1613 · `RPAR`:L1614 · `LSQB`:L1615
`RSQB`:L1616 · `LBRACE`:L1617 · `RBRACE`:L1618 · `COLON`:L1619 · `SEMI`:L1620 · `COMMA`:L1621 · `DOT`:L1622
`AT`:L1623 · `PLUS`:L1624 · `MINUS`:L1625 · `STAR`:L1626 · `SLASH`:L1627 · `PERCENT`:L1628 · `VBAR`:L1629
`AMPER`:L1630 · `CIRCUMFLEX`:L1631 · `TILDE`:L1632 · `LESS`:L1633 · `GREATER`:L1634 · `EQUAL`:L1635
`QUESTION`:L1636 · `EXCLAMATION`:L1637 · `DOLLAR`:L1638 · `BACKQUOTE`:L1639 · `UNDERSCORE`:L1640

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

### `Analysis/Comptime/PilotComptimeEvaluator.cs` (557 lines)
`EvaluateProgram`:L62 · `VisitFunc_def`:L91 · `VisitConst_stmt`:L120 · `VisitLet_stmt`:L149 · `VisitVar_stmt`:L169
`VisitComptime_stmt`:L183 · `VisitComptime_expr`:L202 · `VisitAssert_stmt`:L221 · `VisitAtom_expr`:L263
`VisitTest`:L330 · `ValidateComptimePurity`:L355 · `ReportComptimeFailure`:L370
`IsComptimeConstantExpression`:L381 · `AreAllCallArgumentsConstant`:L389 · `ReferencesDynamicRuntimeLocal`:L402
`ContainsImpureCall`:L422 · `ContainsImpureCall`:L424 · `TryExecuteInSandbox`:L453
`TryEvaluateNodeInSandbox`:L468 · `ExecuteComptimeBlockViaPilVm`:L502 · `TryUnwrapSingleAtomExpr`:L524

### `Analysis/Comptime/PilotStaticDemotionPass.cs` (887 lines)
`PilotDemotedVariableCoordinate`:L23 · `PilotDemotedFieldAccess`:L29 · `PilotDemotedProjectionPlan`:L36
`PilotFunctionAuditSummary`:L43 · `GetOrAllocateSlot`:L156 · `Analyze`:L183 · `Run`:L194
`PrescanNominalLayoutsAndDeclarations`:L221 · `VisitFunc_def`:L302 · `VisitDynamic_stmt`:L386
`VisitLet_stmt`:L406 · `VisitVar_stmt`:L412 · `VisitConst_stmt`:L418 · `RegisterPatternDeclaration`:L424
`VisitWith_stmt`:L466 · `VisitTerm`:L489 · `VisitAtom_expr`:L533 · `VisitInline_foreign_block`:L669
`TryResolveLexicalCoordinate`:L685 · `ResolveAtomStaticTypeName`:L725 · `ResolveFactorStaticTypeName`:L743
`TryInferConstructedTypeName`:L759 · `StripGenericsAndState`:L776 · `IndexOfOrdinal`:L783
`ExtractPatternIdentifiers`:L793 · `HasZeroRuntimePragmaOrDecorator`:L830 · `GetDecoratorFlags`:L847
`FinalizeFrame`:L866

### `Analysis/ContractVerification/PilotContractVerifier.cs` (353 lines)
`IsFunctionContractFullyDischarged`:L29 · `VisitProgram`:L50 · `VisitConst_stmt`:L72 · `VisitLet_stmt`:L83
`VisitVar_stmt`:L94 · `VisitFunc_def`:L104 · `VisitAtom_expr`:L144 · `VisitAssert_stmt`:L164
`VerifyCallPreconditions`:L190 · `TryEvaluateConstantInt`:L248 · `TryEvaluatePrecondition`:L255
`TryEvaluateConstantBool`:L321

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

### `Analysis/TypeSystem/PilotBidirectionalInferencer.cs` (4400 lines)
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
`AnalyzeCompoundStmt`:L1297 · `AnalyzeVarDecl`:L1349 · `AnalyzeVarDecl`:L1360 · `BindDestructuringPattern`:L1423
`AnalyzeGuardStmt`:L1453 · `AnalyzeIfStmt`:L1530 · `AnalyzeMatchStmt`:L1674 · `AnalyzeForStmt`:L1775
`CollectLoopBodyFlowTypes`:L1872 · `AnalyzeFlowStmt`:L1906 · `AnalyzeExprStmt`:L1990
`SubtractAdtOrUnionVariant`:L2202 · `TryExtractPatternNarrowedType`:L2222 · `BindMatchPatternVariables`:L2283
`FindFirstPrimaryPattern`:L2316 · `Check`:L2331 · `CheckAtom`:L2419 · `CheckLambda`:L2480
`ExtractLambdaParameters`:L2573 · `Synthesize`:L2610 · `SynthesizeTest`:L2644 · `SynthesizePipeExpr`:L2671
`SynthesizePipeStage`:L2683 · `SynthesizeSetQueryExpr`:L2699 · `SynthesizeCoalesceExpr`:L2707
`SynthesizeOrTest`:L2757 · `SynthesizeAndTest`:L2765 · `SynthesizeNotTest`:L2773 · `SynthesizeComparison`:L2783
`SynthesizeRangeExpr`:L2804 · `SynthesizeBitorExpr`:L2814 · `SynthesizeBitxorExpr`:L2822
`SynthesizeBitandExpr`:L2830 · `SynthesizeShiftExpr`:L2838 · `SynthesizeArithExpr`:L2846 · `SynthesizeTerm`:L2860
`CombineArithmeticOperandTypes`:L2880 · `ConstrainTypeVariable`:L2908 · `SynthesizeFactor`:L2930
`SynthesizePower`:L2958 · `SynthesizeAtomExpr`:L2969 · `SynthesizeCascadeStep`:L3143
`BindCascadePatternInInference`:L3251 · `InferBuiltinCascadeMemberReturnType`:L3278
`TryRecordRowPolymorphicFieldAccess`:L3296 · `TryInferCollectionHigherOrderMethod`:L3322
`ResolveMemberMethodCall`:L3475 · `ResolveDirectCall`:L3536 · `ResolveSingleSchemeCall`:L3708
`VerifyAndBindRowPolymorphicArgument`:L3780 · `SelectBestFunctionOverload`:L3825 · `SynthesizeAtom`:L3918
`VerifyAndUnifyConstraint`:L4058 · `RecordConstraintFailure`:L4083 · `AddInlayHint`:L4112 · `LookupFieldType`:L4127
`TryLookupFieldAndOffset`:L4134 · `JoinTypes`:L4210 · `InferSimpleSuiteReturnType`:L4249
`TryExtractFailArgumentType`:L4267 · `SuiteAlwaysExits`:L4292 · `SimpleStmtAlwaysExits`:L4315
`FindIfBranchPattern`:L4332 · `HasIfBranchSome`:L4348 · `FindFirstAtomExpr`:L4367 · `IsNullLiteralText`:L4378
`IsSimpleIdentifier`:L4381 · `IsKnownBuiltinTypeName`:L4393 · `TruncateSource`:L4396

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

### `Cli/Program.cs` (1380 lines)
`Main`:L20 · `PrintVersion`:L271 · `ExecuteInlineScript`:L282 · `ExecuteBuildSubcommand`:L295
`ExecuteCompilationSubcommand`:L411 · `RunScriptFile`:L497 · `ExecuteTestSubcommand`:L631
`ExecuteBenchmarkSubcommand`:L734 · `ExecutePatchSubcommand`:L794 · `ExecuteWatchSubcommand`:L876
`ExecutePackageSubcommand`:L921 · `ExecuteVenvSubcommand`:L1015 · `PrintUsage`:L1090 · `ParseAndReport`:L1128
`ExecuteCheckSubcommand`:L1238

### `CodeGen/Engine/PilotCodeGeneratorPipeline.cs` (410 lines)
`ShouldRun`:L39 · `Run`:L59 · `Run`:L73 · `ExtractScriptModel`:L165 · `ExtractClassModel`:L212
`ExtractMethodModel`:L287 · `ExtractDecorator`:L326 · `ExtractViaRegexFallback`:L352
`ExecuteInScriptGenerators`:L381

### `Compiler/PilotBytecodeCompiler.cs` (3203 lines)
`IsImmutable`:L43 · `AllocateTempSlot`:L50 · `RegisterBindingMetadata`:L53 · `HasDeclaredBinding`:L73
`EmitStoreBinding`:L84 · `TryResolveUpvalueSlot`:L99 · `EmitRestoreBinding`:L121
`EmitLoadBindingOrUndefined`:L138 · `EmitLiteralConstant`:L161 · `TryResolveAdtVariant`:L183
`TryResolveAdtType`:L230 · `TryResolveRecordType`:L245 · `IsKnownClassOrStructName`:L260 · `Compile`:L286
`CompileSuite`:L313 · `PrescanProgramTypeDeclarations`:L365 · `RegisterAdtCompileTime`:L405
`RegisterRecordCompileTime`:L414 · `TryBuildTypeAliasAdtDescriptor`:L419 · `BuildEnumDescriptor`:L507
`BuildRecordDescriptor`:L535 · `ExtractGenericParamList`:L553 · `ExtractSerializedFields`:L559
`VisitProgram`:L596 · `VisitStmt`:L610 · `VisitSimple_stmt`:L619 · `VisitSmall_stmt`:L634
`VisitCompound_stmt`:L655 · `VisitType_alias_stmt`:L684 · `VisitEnum_def`:L705 · `VisitRecord_def`:L723
`VisitMatch_stmt`:L741 · `VisitGuard_stmt`:L747 · `VisitSuite`:L753 · `IsSliceBufferTypeAnnotation`:L775
`IsTopLevelForExpression`:L787 · `VisitLet_stmt`:L796 · `VisitVar_stmt`:L824 · `VisitConst_stmt`:L852
`TryCompileIrrefutableDestructuring`:L880 · `VisitAssert_stmt`:L917 · `VisitComptime_stmt`:L951
`VisitComptime_expr`:L993 · `IsPureContextWithStmt`:L1024 · `VisitWith_stmt`:L1039
`ExtractBindingIdentifiers`:L1069 · `CompileVariableDeclaration`:L1115 · `VisitExpr_stmt`:L1148
`AssignToTarget`:L1202 · `CompileAugmentedAssign`:L1279 · `EmitAugassignOp`:L1415 · `VisitFlow_stmt`:L1433
`VisitIf_stmt`:L1479 · `VisitWhile_stmt`:L1537 · `VisitFor_stmt`:L1586 · `VisitFunc_def`:L1659 · `VisitTest`:L1714
`VisitPipe_expr`:L1741 · `TryCompileFluentPipeStage`:L1762 · `VisitSet_query_expr`:L1832
`VisitCoalesce_expr`:L1841 · `CompileCoalesceExprPrefix`:L1847 · `CompileCatchBlockAsExpression`:L1910
`VisitOr_test`:L1928 · `VisitAnd_test`:L1945 · `VisitNot_test`:L1962 · `VisitComparison`:L1981
`VisitRange_expr`:L2019 · `VisitBitor_expr`:L2036 · `VisitBitxor_expr`:L2050 · `VisitBitand_expr`:L2064
`VisitShift_expr`:L2078 · `VisitArith_expr`:L2093 · `VisitTerm`:L2108 · `VisitFactor`:L2177 · `VisitPower`:L2207
`VisitAtom_expr`:L2220 · `RequiresAtomExprAstFallback`:L2354 · `RequiresCascadeAstFallback`:L2415
`PreallocateCascadeFallbackBindings`:L2482 · `PreallocateSingleCascadeBindings`:L2493
`CompileCascadeTrailer`:L2530 · `EmitCascadeCaptureStore`:L2627 · `HasNamedArguments`:L2691
`TryResolveKnownFieldOrderForCall`:L2702 · `OrderCallArgumentExpressions`:L2742 · `VisitAtom`:L2790
`VisitLiteral`:L2959 · `VisitCollection_expr`:L3090 · `IsSimpleIdentifier`:L3123 · `GetAtomExpr`:L3139
`CompileAtomExprPrefix`:L3147 · `UnescapeStringLiteral`:L3172 · `StripQuotes`:L3188

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

### `Pilot.LanguageServer/Semantics/PilotDiagnosticCatalog.cs` (366 lines)
`Format`:L37 · `GetDescriptor`:L362 · `GetAllDescriptors`:L365

### `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (5274 lines)
`IsBuiltinType`:L51 · `Analyze`:L347 · `SynchronizeInferredTypesFromAnalysis`:L426 · `SynchronizeScopeSymbols`:L552
`FindFunctionSymbol`:L593 · `FindFunctionScope`:L605 · `FormatInferredTypeForSymbol`:L617
`CheckUnusedVariables`:L638 · `PreRegisterTopLevelDeclarations`:L675 · `RegisterFunctionStub`:L765
`RegisterTypeStub`:L790 · `DeclareBindingPattern`:L814 · `VisitLet_stmt`:L873 · `VisitVar_stmt`:L888
`VisitConst_stmt`:L923 · `VisitRef_stmt`:L938 · `VisitDynamic_stmt`:L953 · `VisitAnnassign_stmt`:L961
`VisitType_alias_stmt`:L992 · `VisitPlainTypeAlias`:L1001 · `VisitExpr_stmt`:L1014
`CheckImmutableMemberAssignment`:L1124 · `StripGenericSuffixForLookup`:L1179 · `FindFirstAtomExpr`:L1186
`VisitGuard_stmt`:L1197 · `VisitDefer_stmt`:L1264 · `VisitFunc_def`:L1270 · `VisitContract_clause`:L1423
`VisitPattern_assign_stmt`:L1436 · `VisitClass_def`:L1445 · `VisitStruct_def`:L1477 · `VisitInterface_def`:L1506
`VisitEnum_def`:L1522 · `VisitActor_def`:L1564 · `VisitModel_def`:L1580 · `VisitProperty_def`:L1597
`VisitOperator_def`:L1672 · `VisitExtension_def`:L1711 · `VisitAdapter_def`:L1732 · `VisitUnit_stmt`:L1762
`VisitDsl_stmt`:L1778 · `VisitMacro_def`:L1794 · `VisitQuote_stmt`:L1825 · `VisitQuote_expr`:L1835
`VisitComptime_stmt`:L1844 · `VisitComptime_expr`:L1862 · `VisitQuasiquote_expr`:L1884 · `VisitSplice_expr`:L1896
`VisitForeign_import_stmt`:L1908 · `VisitExtern_def`:L2003 · `ProcessExternMember`:L2066
`ProcessExternFunction`:L2082 · `ProcessExternType`:L2160 · `VisitInline_foreign_block`:L2214
`VisitDatabase_stmt`:L2232 · `VisitTransaction_stmt`:L2243 · `VisitSavepoint_stmt`:L2256
`VisitSavepoint_block`:L2267 · `VisitRollback_stmt`:L2279 · `VisitNoalloc_stmt`:L2301 · `VisitWhile_stmt`:L2311
`VisitFor_stmt`:L2345 · `VisitIf_stmt`:L2448 · `VisitFlow_stmt`:L2468 · `VisitPass_stmt`:L2620
`VisitAssert_stmt`:L2627 · `VisitComparison`:L2636 · `VisitRange_expr`:L2656 · `VisitTest`:L2665
`VisitComp_op`:L2682 · `VisitSelect_stmt`:L2690 · `VisitSelect_arm`:L2709 · `VisitQuery_expr`:L2733
`VisitSql_query_expr`:L2782 · `IsInsideSyncFunction`:L2810 · `VisitUnary_op`:L2827 · `VisitPrimary_type`:L2859
`VisitAtom_expr`:L2903 · `VisitTerm`:L3083 · `VisitLabeled_stmt`:L3109 · `VisitBlock`:L3119
`VisitLambda_expr`:L3128 · `VisitMacro_call`:L3212 · `VisitSymbol_literal`:L3224 · `VisitCascade_trailer`:L3240
`VisitNameof_expr`:L3347 · `VisitAtom`:L3366 · `InferLiteralType`:L3563 · `ValidateAndResolveType`:L3586
`IsCompatibleType`:L3614 · `IsStrictPrimitive`:L3721 · `NormalizePrimitive`:L3727 · `TryExtractUnit`:L3735
`ExpandUnitExpression`:L3748 · `SuiteGuaranteesExit`:L3759 · `SimpleStmtExits`:L3772 · `VisitSimple_stmt`:L3779
`VisitSuite`:L3823 · `StmtGuaranteesExit`:L3869 · `VisitArith_expr`:L3885 · `VisitExcept_clause`:L3925
`VisitMatch_stmt`:L3946 · `VisitMatch_arm`:L3969 · `VisitMatchArmWithScrutinee`:L3974
`BindPatternIdentifiers`:L4016 · `VisitRecord_def`:L4022 · `RegisterAdtSumTypeAlias`:L4080
`RegisterAdtVariantSymbol`:L4121 · `FinishGuardSuite`:L4159 · `VisitIfBranchesWithPatterns`:L4177
`BindPatternVarSingle`:L4286 · `BindPatternNode`:L4309 · `BindPatternNodeLegacyUnused`:L4477
`RegisterGenericParams`:L4512 · `DeclareVariableSymbol`:L4524 · `DeclareVariableSymbolFromToken`:L4539
`CreateSymbol`:L4573 · `CreateSymbolFromToken`:L4595 · `RegisterSymbol`:L4645 · `AddSymbolReferenceDirect`:L4672
`AddSemanticTokenDirect`:L4684 · `ProcessFStringNode`:L4705 · `AddSymbolReference`:L4843 · `AddSemanticToken`:L4856
`AddDiagnostic`:L4869 · `AddDiagnosticDirect`:L4892 · `GetSemanticTokenTypeForSymbol`:L4914
`GetTokenCoords`:L4931 · `ExtractDictTypes`:L4940 · `ExtractElementType`:L4968 · `IsSubtypeOf`:L5011
`FindCommonBaseType`:L5024 · `HomogenizeTypes`:L5058 · `TryExtractSimpleIdentifier`:L5131
`FindFirstIdentifier`:L5140 · `Truncate`:L5151 · `EnterScope`:L5154 · `Dispose`:L5200 · `SyntaxError`:L5208

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

### `Runtime/Bytecode/PilotBytecodeChunk.cs` (638 lines)
`Emit`:L89 · `EmitJump`:L97 · `PatchJump`:L107 · `AddJumpTable`:L121 · `AddTypeDescriptor`:L130
`AddConstant`:L139 · `AddIdentifier`:L155 · `AddNestedChunk`:L171 · `AddAstFallback`:L180
`GetOrAllocateLocalSlot`:L189 · `TryGetLocalSlot`:L203 · `Write`:L210 · `Read`:L304 · `ReadV2Extensions`:L392
`WriteValue`:L453 · `ReadValue`:L543 · `ReadKeywordVariant`:L568 · `ReadList`:L577 · `ReadMap`:L590
`ReadSet`:L604 · `ReadTuple`:L617 · `ReadRange`:L628

### `Runtime/Bytecode/PilVirtualMachine.cs` (1772 lines)
`Execute`:L29 · `SyncLocalsToScopeIfNeeded`:L1374 · `DeclareTypeDescriptorInContext`:L1396 · `RefreshLayouts`:L1449
`CheckRuntimeTypeMatch`:L1461 · `MatchesClassHierarchy`:L1564 · `EnsureStackCapacity`:L1585
`InvokeCallable`:L1597 · `InvokeMethod`:L1636 · `CreateIterator`:L1653 · `EnumerateSliceBuffer`:L1696
`EnumerateRange`:L1704 · `EnumerateString`:L1718 · `EnumerateObjects`:L1726 · `EnumerateSingle`:L1734
`CreateParameters`:L1739

### `Runtime/Effects/PilotEffectDryRunEngine.cs` (808 lines)
`ToPilotObject`:L61 · `ToString`:L88 · `Equals`:L109 · `Equals`:L112 · `GetHashCode`:L114 · `Inspect`:L165
`Commit`:L182 · `Rollback`:L198 · `ToPilotObject`:L207 · `NotifyObjectCreated`:L269 · `NotifyScopeCreated`:L279
`EvaluateEffectDryRun`:L291 · `ExecuteCommitActions`:L350 · `TryReadObjectField`:L370
`TryRecordObjectFieldWrite`:L394 · `TryReadScopeVariable`:L430 · `TryRecordScopeVariableWrite`:L453
`GetReadableList`:L491 · `TryRecordListIndexWrite`:L505 · `TryRecordListAppend`:L545 · `TryReadMapEntry`:L572
`TryRecordMapWrite`:L587 · `TryReadSliceBufferElement`:L614 · `TryRecordSliceBufferWrite`:L630
`TryReadNativeArrayElement`:L653 · `TryRecordNativeArrayWrite`:L669 · `TryRecordDatabaseExec`:L696
`TryReadFileContent`:L720 · `TryRecordFileWrite`:L737 · `TryRecordFileDelete`:L776

### `Runtime/Execution/PilotEngine.cs` (650 lines)
`Execute`:L21 · `Execute`:L27 · `ExecuteFile`:L153 · `EvaluateExpression`:L168 · `InvokeFunction`:L186
`CreateGenerator`:L341 · `CreateGenerator`:L381 · `RunnerBody`:L386 · `Next`:L441 · `Close`:L471
`TranspileTo`:L486 · `TranspileFrom`:L492 · `Compile`:L498 · `CompileFile`:L508 · `CreateCallScope`:L518
`SnapshotOldCalls`:L581 · `Write`:L627 · `Write`:L633 · `WriteLine`:L639 · `Flush`:L645

### `Runtime/Execution/PilotInterpreterVisitor.cs` (5915 lines)
`VisitProgram`:L84 · `VisitStmt`:L129 · `VisitSimple_stmt`:L143 · `VisitSuite`:L160 · `VisitLet_stmt`:L184
`VisitVar_stmt`:L192 · `VisitConst_stmt`:L200 · `EvaluateDeclarationInitializer`:L208 · `TryUnwrapForStmt`:L283
`DeclareBinding`:L293 · `DeclareRecordPattern`:L321 · `ProcessRecordRestPattern`:L349
`ProcessRecordFieldPattern`:L381 · `DeclareListPattern`:L443 · `DeclareTuplePattern`:L516
`VisitPattern_assign_stmt`:L596 · `VisitExpr_stmt`:L614 · `TryUnwrapAtomExpr`:L691 · `AnalyzeTarget`:L731
`EvaluateAtomExprPrefix`:L808 · `AssignTarget`:L895 · `VisitFunc_def`:L1030 · `ContainsYield`:L1131
`ContainsYieldCore`:L1137 · `VisitClass_def`:L1154 · `VisitState_def`:L1319 · `VisitMacro_def`:L1372
`VisitExtern_def`:L1407 · `VisitType_alias_stmt`:L1461 · `VisitEnum_def`:L1524 · `VisitRecord_def`:L1543
`VisitStruct_def`:L1582 · `DefaultValueForType`:L1643 · `RegisterAdtDefinition`:L1652
`RefreshAllAdtAndRecordLayouts`:L1675 · `IsKnownTypeOrBinding`:L1687 · `ExtractGenericParamNames`:L1703
`ExtractAdtFields`:L1709 · `VisitExtension_def`:L1738 · `EnsureSelfParameter`:L1851
`SanitizeExtensionTargetName`:L1864 · `VisitAdapter_def`:L1874 · `VisitSteal_stmt`:L1921
`VisitDelegate_stmt`:L1961 · `VisitContract_forward_stmt`:L1982 · `VisitIf_stmt`:L2007
`HasBranchSomeKeyword`:L2087 · `FindBranchPattern`:L2107 · `FindBranchPatternGuard`:L2126
`IsBareOptionalBindingPattern`:L2145 · `VisitWhile_stmt`:L2170 · `VisitLabeled_stmt`:L2296 · `VisitBlock`:L2327
`VisitFor_stmt`:L2350 · `TryGetSimpleForTargetName`:L2797 · `VisitWith_stmt`:L2815 · `AssignForTarget`:L2896
`VisitMatch_stmt`:L2922 · `EvaluateMatchSuite`:L2951 · `VisitGuard_stmt`:L2978 · `EvaluateGuardRhs`:L3077
`VisitDefer_stmt`:L3124 · `IsKnownExceptionTypeName`:L3181 · `VisitTry_stmt`:L3191 · `IsExceptionMatching`:L3308
`VisitAssert_stmt`:L3332 · `VisitComptime_stmt`:L3347 · `VisitComptime_expr`:L3360
`EvaluateComptimeStatements`:L3369 · `VisitFlow_stmt`:L3408 · `VisitTestlist`:L3537 · `UnwrapAndVisit`:L3554
`VisitTest`:L3601 · `VisitLambda_expr`:L3631 · `VisitPipe_expr`:L3708 · `VisitSet_query_expr`:L3731
`VisitCoalesce_expr`:L3744 · `VisitCoalesceExprPrefix`:L3753 · `EvaluateCatchClause`:L3791 · `VisitOr_test`:L3828
`VisitAnd_test`:L3843 · `VisitNot_test`:L3858 · `VisitComparison`:L3875 · `VisitRange_expr`:L3943
`VisitBitor_expr`:L3956 · `VisitBitxor_expr`:L3970 · `VisitBitand_expr`:L3985 · `VisitShift_expr`:L3999
`VisitArith_expr`:L4017 · `VisitTerm`:L4035 · `EvaluateProjection`:L4089 · `VisitFactor`:L4149
`VisitUnary_op`:L4159 · `VisitPower`:L4198 · `VisitAtom_expr`:L4213 · `CreateFluentMemberFunction`:L4388
`TryInvokeDirectMemberCall`:L4430 · `EvaluateCascade`:L4561 · `ApplyHoistedCascades`:L4706
`BindCascadeCapturedValue`:L4731 · `DispatchCall`:L4754 · `InvokeCallableSingle`:L4898
`ExtractCollectionItems`:L5018 · `EvaluateCallArguments`:L5073 · `EvaluateArglist`:L5079
`BindArgumentsToFunction`:L5084 · `IsCollectionOrTupleParameterType`:L5164 · `VisitAtom`:L5194
`VisitSymbol_literal`:L5230 · `VisitMacro_call`:L5251 · `IsBuiltinSpecialFormName`:L5268
`ResolveIdentifier`:L5273 · `VisitLiteral`:L5293 · `ParseDurationLiteralSeconds`:L5402
`ParseFrequencyLiteralHz`:L5420 · `ParseMemoryLiteralBytes`:L5434 · `IsSimpleIdentifier`:L5450
`ParseFStringSegments`:L5463 · `EvaluateFString`:L5509 · `UnescapeString`:L5553 · `VisitCollection_expr`:L5566
`VisitDictorsetmaker`:L5597 · `VisitTestlist_comp`:L5680 · `VisitInline_foreign_block`:L5696
`GetBlockBodyText`:L5821 · `VisitInsert_stmt`:L5853 · `VisitUpdate_stmt`:L5880 · `VisitDelete_stmt`:L5903

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

### `Runtime/Reactive/PilotSignalGraph.cs` (820 lines)
`Read`:L40 · `Subscribe`:L42 · `NotifyWatchers`:L48 · `ToPilotObject`:L61 · `ResolveDynamicMember`:L114
`SynchronizeObjectFields`:L128 · `Read`:L176 · `Set`:L191 · `Read`:L263 · `Recompute`:L265 · `Compare`:L290
`Equals`:L311 · `Equals`:L315 · `GetHashCode`:L317 · `Equals`:L332 · `Equals`:L336 · `GetHashCode`:L338
`CreateSourceSignal`:L357 · `GetOrCreatePropertySignal`:L360 · `GetOrCreateVariableSignal`:L376
`NotifyObjectPropertyChanged`:L393 · `NotifyScopeVariableChanged`:L408 · `Batch`:L426 · `Batch`:L443
`PropagateFrom`:L460 · `FlushDirtyQueue`:L478 · `TryExtractSignalNode`:L506 · `CreateReactiveCallSignal`:L523
`EvaluateReactiveArgumentExpr`:L606 · `IsSimpleIdentifier`:L621 · `ExtractObservableDependenciesFromTree`:L632
`EvaluateAtomAndTrailersPrefix`:L736 · `HasStaticObservableInput`:L766 · `TreeContainsStaticObservable`:L783

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

### `Runtime/Values/PilotObject.cs` (768 lines)
`RegisterFieldSlot`:L25 · `Union`:L45 · `Subtract`:L73 · `Subtract`:L107 · `Intersect`:L121 · `CloneWithName`:L148
`MergeMethodValue`:L165 · `CloneSnapshot`:L223 · `RestoreFromSnapshot`:L241 · `StealFrom`:L309
`DelegateContract`:L344 · `Instantiate`:L359 · `PopulateDefaultFields`:L399 · `ToString`:L416 · `GetFieldSlot`:L447
`SetFieldSlotDirect`:L472 · `SetFieldSlot`:L489 · `GetMember`:L511 · `SetMember`:L663 · `BindMethodValue`:L681
`BindMethod`:L714 · `Clone`:L716 · `Equals`:L726 · `Equals`:L740 · `GetHashCode`:L742 · `ToString`:L754

### `Runtime/Values/PilotValue.cs` (2419 lines)
`CreateTable`:L38 · `FromBool`:L63 · `FromInt`:L64 · `FromUInt`:L65 · `FromFloat`:L66 · `FromDecimal`:L67
`FromChar`:L68 · `FromString`:L69 · `FromKeyword`:L70 · `FromKeyword`:L71 · `FromList`:L72 · `FromFrozenList`:L73
`FromTuple`:L74 · `FromMap`:L75 · `FromFrozenMap`:L76 · `FromSet`:L77 · `FromRange`:L78 · `FromMatrix`:L79
`FromFunction`:L80 · `FromObject`:L81 · `FromGenerator`:L82 · `FromLazy`:L83 · `FromFuture`:L84 · `FromForeign`:L85
`FromMethodGroup`:L86 · `FromNativeArray`:L87 · `FromNativeMap`:L88 · `FromSliceBuffer`:L89 · `FromAdtType`:L90
`FromAdtUnit`:L91 · `FromAdtBoxed`:L92 · `FromRecordUnboxedEmpty`:L93 · `FromRecordUnboxedPairF64`:L94
`FromRecordBoxed`:L96 · `FromKeywordVariant`:L97 · `FromSome`:L105 · `UnwrapOption`:L122 · `FromFailure`:L142
`UnwrapFailure`:L162 · `TryCreateUnboxedAdtSingle`:L195 · `PilotValue`:L218 · `PilotValue`:L219 · `PilotValue`:L220
`PilotValue`:L221 · `PilotValue`:L222 · `PilotValue`:L223 · `PilotValue`:L224 · `PilotValue`:L225
`PilotValue`:L226 · `AsInt`:L255 · `AsFloat`:L267 · `AsString`:L279 · `AsDecimal`:L280 · `AsChar`:L281
`AsKeyword`:L282 · `AsList`:L284 · `AsTuple`:L285 · `AsMap`:L286 · `AsSet`:L287 · `AsRange`:L288 · `AsMatrix`:L289
`AsFunction`:L290 · `AsObject`:L291 · `AsGenerator`:L292 · `AsLazy`:L293 · `AsFuture`:L294 · `AsMethodGroup`:L295
`AsNativeArray`:L296 · `AsNativeMap`:L297 · `AsSliceBuffer`:L298 · `AsForeign`:L299 · `TryGetFrozenMap`:L309
`Freeze`:L325 · `AsAdtType`:L381 · `AsKeywordVariant`:L382 · `GetAdtVariantDefinition`:L384 · `GetAdtTag`:L392
`GetAdtFieldCount`:L394 · `GetAdtField`:L396 · `GetRecordDefinition`:L419 · `GetRecordFieldCount`:L426
`GetRecordField`:L428 · `CreateClassConstructor`:L448 · `GetMember`:L806 · `CreateParseError`:L1441
`ComputeCollectionSum`:L1450 · `InvokeSingleCallback`:L1473 · `MapCollection`:L1490 · `FilterCollection`:L1500
`TryInvokePrimitiveIntrinsic`:L1518 · `EnumerateCollection`:L2038 · `CreateAppendFunction`:L2046
`ResolveExtensionOrRoot`:L2093 · `ResolveRootMember`:L2131 · `CloneValue`:L2184 · `Equals`:L2200
`EqualsAdt`:L2260 · `EqualsRecord`:L2275 · `ComputeAdtHashCode`:L2290 · `ComputeRecordHashCode`:L2300
`FormatAdt`:L2309 · `FormatRecord`:L2319 · `EqualsList`:L2333 · `EqualsTuple`:L2340 · `EqualsMap`:L2347
`Equals`:L2357 · `GetHashCode`:L2359 · `ToString`:L2374 · `FormatNestedSome`:L2411

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
- `Analysis/Comptime/PilotComptimeEvaluator.cs` (557 L) — class `PilotComptimeEvaluator`:L21
- `Analysis/Comptime/PilotStaticDemotionPass.cs` (887 L) — enum `PilotDemotedBindingKind`:L13, record `PilotDemotedVariableCoordinate`:L23, record `PilotDemotedFieldAccess`:L29, record `PilotDemotedProjectionPlan`:L36, record `PilotFunctionAuditSummary`:L43, class `PilotStaticDemotionReport`:L73, class `PilotStaticDemotionPass`:L116, class `LexicalFrame`:L123
- `Analysis/ContractVerification/PilotContractVerifier.cs` (353 L) — class `PilotContractVerifier`:L16, class `FunctionContractInfo`:L32
- `Analysis/DimensionalAnalysis/PilotDimensionChecker.cs` (344 L) — class `UnitDimensionSignature`:L13, class `PilotDimensionChecker`:L101
- `Analysis/Loops/PilotForLoopAnalyzer.cs` (473 L) — enum `LoopScheduleMode`:L11, enum `LoopReturnMode`:L26, class `PilotForLoopInfo`:L41, class `PilotForLoopAnalyzer`:L58, class `ScopedBodyScanState`:L453
- `Analysis/Ownership/PilotBorrowChecker.cs` (503 L) — enum `ResourceState`:L13, enum `ResourceAffinity`:L25, class `ResourceBinding`:L44, class `PilotBorrowChecker`:L68
- `Analysis/PatternMatching/PilotExhaustivenessChecker.cs` (2046 L) — class `PilotExhaustivenessChecker`:L21, record `MatchPat`:L536, record `WildPat`:L537, record `OrPat`:L541, record `CtorPat`:L542, enum `CtorFamily`:L544, record `CtorDesc`:L563
- `Analysis/PilotAnalysisPipeline.cs` (166 L) — class `PilotStaticAnalysisResult`:L18, class `PilotAnalysisPipeline`:L58, class `NullAssertionSyntaxListener`:L142
- `Analysis/TypeSystem/PilotBidirectionalInferencer.cs` (4400 L) — class `PilotSourceSpan`:L14, class `PilotTypeConstraint`:L66, enum `PilotInlayHintKind`:L99, class `PilotInlayHint`:L109, class `PilotMonomorphizedCallSite`:L142, class `PilotFunctionScheme`:L178, class `PilotNominalTypeLayout`:L245, class `PilotBidirectionalInferencer`:L278
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
- `antlr/Pilot.g4` (1723 L)
- `antlr/realtime/RealtimeLexer.g4` (34 L)
- `antlr/realtime/RealtimeParser.g4` (32 L)
- `antlr/udsa/UDSALexer.g4` (19 L)
- `antlr/udsa/UDSAParser.g4` (37 L)

### Cli
- `Cli/PilotRepl.cs` (300 L) — class `PilotRepl`:L16
- `Cli/Program.cs` (1380 L) — class `Program`:L18

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
- `Compiler/PilotBytecodeCompiler.cs` (3203 L) — class `PilotBytecodeCompiler`:L17, class `LoopContext`:L263
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
- `examples/showcase/24_comptime_and_zero_runtime.pilot` (128 L)
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
- `Pilot.LanguageServer/Semantics/PilotDiagnosticCatalog.cs` (366 L) — class `PilotDiagnosticDescriptor`:L9, class `PilotDiagnosticCatalog`:L54
- `Pilot.LanguageServer/Semantics/PilotSemanticAnalyzer.cs` (5274 L) — class `PilotSemanticAnalyzer`:L10, class `ScopeRestorer`:L5191, class `LspErrorListener`:L5203
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
- `Pilot.Tests/PilotComptimeAndStaticDemotionTests.cs` (465 L) — class `PilotComptimeAndStaticDemotionTests`:L28
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
- `Pilot.Tests/PilotSemanticCallProjectionsTests.cs` (298 L) — class `PilotSemanticCallProjectionsTests`:L22
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
- `Runtime/Bytecode/PilotBytecodeChunk.cs` (638 L) — class `PilotJumpTable`:L12, class `PilotSerializedFieldDescriptor`:L29, class `PilotSerializedVariantDescriptor`:L38, class `PilotBytecodeTypeDescriptor`:L48, class `PilotBytecodeChunk`:L62
- `Runtime/Bytecode/PilotInstruction.cs` (46 L) — struct `PilotInstruction`:L11
- `Runtime/Bytecode/PilotOpcode.cs` (330 L) — enum `PilotOpcode`:L11
- `Runtime/Bytecode/PilVirtualMachine.cs` (1772 L) — class `PilVirtualMachine`:L21
- `Runtime/Concurrency/PilotFuture.cs` (72 L) — class `PilotFuture`:L11
- `Runtime/Contracts/PilotContractRegistry.cs` (313 L) — class `PilotContractRegistry`:L14
- `Runtime/ControlFlow/PilotControlFlowSignals.cs` (99 L) — enum `PilotFlowSignalKind`:L9, class `PilotControlFlowSignal`:L21, class `PilotReturnSignal`:L29, class `PilotBreakSignal`:L42, class `PilotContinueSignal`:L55, class `PilotYieldSignal`:L68, class `PilotRuntimeException`:L81
- `Runtime/ControlFlow/PilotExceptions.cs` (123 L) — class `PilotTypeException`:L9, class `PilotDivideByZeroException`:L18, class `PilotIndexOutOfRangeException`:L27, class `PilotKeyNotFoundException`:L36, class `PilotNullReferenceException`:L45, class `PilotAssertionException`:L54, class `PilotRecursionException`:L63, class `PilotContractException`:L72, class `PilotTimeoutException`:L81, class `PilotPatternMatchingException`:L90, class `PilotContextException`:L99, class `PilotUserException`:L108
- `Runtime/Effects/PilotEffectDryRunEngine.cs` (808 L) — enum `EffectDiffKind`:L19, class `PilotEffectDiff`:L33, class `ShadowEffectContext`:L96, struct `ObjectFieldKey`:L98, class `PilotEffectPlan`:L144, class `PilotEffectDryRunEngine`:L256
- `Runtime/Environment/PilotBinding.cs` (22 L) — class `PilotBinding`:L8
- `Runtime/Environment/PilotEnvironment.cs` (59 L) — class `PilotEnvironment`:L9
- `Runtime/Environment/PilotScope.cs` (166 L) — class `PilotScope`:L10
- `Runtime/Execution/IPilotEngine.cs` (56 L) — interface `IPilotEngine`:L10
- `Runtime/Execution/PilotEngine.cs` (650 L) — class `PilotEngine`:L19, class `GeneratorRunner`:L351, class `MergedTextWriter`:L614
- `Runtime/Execution/PilotExecutionContext.cs` (244 L) — class `PilotExecutionContext`:L13
- `Runtime/Execution/PilotExecutionOptions.cs` (87 L) — class `PilotExecutionOptions`:L13
- `Runtime/Execution/PilotExecutionResult.cs` (57 L) — class `PilotExecutionResult`:L10
- `Runtime/Execution/PilotInterpreterVisitor.cs` (5915 L) — class `PilotInterpreterVisitor`:L19, struct `FStringSegment`:L41, enum `TargetAssignmentKind`:L57, class `TargetAssignmentInfo`:L66
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
- `Runtime/Reactive/PilotSignalGraph.cs` (820 L) — class `PilotSignalNode`:L18, class `PilotSourceSignal`:L153, class `PilotComputedSignal`:L233, class `PilotSignalGraph`:L284, class `TopologicalRankComparer`:L286, struct `ObjectPropertyKey`:L300, struct `ScopeVariableKey`:L321
- `Runtime/SelectiveExecution/PilotBenchmarkReport.cs` (47 L) — class `PilotBenchmarkResult`:L9, class `PilotBenchmarkReport`:L37
- `Runtime/SelectiveExecution/PilotSelectiveExecutionEngine.cs` (323 L) — class `PilotSelectiveExecutionEngine`:L20
- `Runtime/SelectiveExecution/PilotTestReport.cs` (46 L) — class `PilotTestCaseResult`:L9, class `PilotTestReport`:L32
- `Runtime/Serialization/PilotJsonSerializer.cs` (198 L) — class `PilotJsonSerializer`:L15
- `Runtime/Values/PilotAdt.cs` (1037 L) — class `PilotAdtFieldDefinition`:L14, record `PilotFieldMemoryLayout`:L35, record `PilotVariantMemoryLayout`:L46, class `PilotAdtMemoryLayout`:L58, class `PilotAdtFloatSlot`:L563, class `PilotAdtVariantDefinition`:L572, class `PilotAdtDefinition`:L721, class `PilotAdtInstance`:L768, class `PilotRecordDefinition`:L828, class `PilotRecordInstance`:L951, class `PilotKeywordVariant`:L1002
- `Runtime/Values/PilotFrozenMap.cs` (206 L) — class `PilotFrozenMap`:L13
- `Runtime/Values/PilotFunction.cs` (319 L) — class `PilotParameter`:L12, class `PilotFunction`:L39
- `Runtime/Values/PilotGenerator.cs` (123 L) — enum `PilotGeneratorState`:L13, class `PilotGenerator`:L25
- `Runtime/Values/PilotKeyword.cs` (301 L) — class `PilotKeyword`:L13
- `Runtime/Values/PilotLazy.cs` (55 L) — class `PilotLazy`:L9
- `Runtime/Values/PilotMatrix.cs` (209 L) — class `PilotMatrix`:L9
- `Runtime/Values/PilotMethodGroup.cs` (635 L) — class `PilotMethodGroup`:L21
- `Runtime/Values/PilotObject.cs` (768 L) — class `PilotClassDefinition`:L11, class `PilotObject`:L422
- `Runtime/Values/PilotRange.cs` (98 L) — class `PilotRange`:L10
- `Runtime/Values/PilotValue.cs` (2419 L) — struct `PilotValue`:L15, class `PilotNestedSomeSentinel`:L27, class `PilotFailurePrimitiveSentinel`:L32, class `PilotNestedFailureBox`:L47
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
- `tests/36_comptime_and_static_demotion.pilot` (114 L)

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
