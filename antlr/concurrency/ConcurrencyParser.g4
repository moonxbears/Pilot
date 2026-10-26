grammar ConcurrencyParser;

import CoreParser, ConcurrencyLexer, CoreLexer;

// =============================================================================
// DOMAIN: Asynchronous & Multi-Threaded Concurrency (Parser)
// =============================================================================

actor_def
    : decorators? ACTOR identifier generic_params? COLON? suite
    ;

spawn_stmt
    : SPAWN (LPAR test RPAR)? COLON? suite
    ;

atomic_stmt
    : ATOMIC (LPAR test? RPAR)? COLON? suite
    ;

sync_stmt
    : (SYNC | LOCK) LPAR test RPAR COLON? suite
    ;

isolate_stmt
    : ISOLATE COLON? suite
    ;

select_stmt
    : SELECT COLON? select_suite
    ;

select_suite
    : LBRACE select_arm* RBRACE
    ;

select_arm
    : CASE select_case_target (COLON | ARROW)? suite
    | TIMEOUT test (COLON | ARROW)? suite
    | DEFAULT (COLON | ARROW)? suite
    ;

select_case_target
    : identifier EQUAL ARROW_LEFT test
    | test ARROW_LEFT test
    | ARROW_LEFT test
    ;

channel_send_stmt
    : test ARROW_LEFT test
    ;
