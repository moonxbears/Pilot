grammar RealtimeParser;

import CoreParser, RealtimeLexer, CoreLexer;

// =============================================================================
// DOMAIN: Soft Real-Time vs Hard Real-Time Systems (Parser)
// =============================================================================

deadline_stmt
    : DEADLINE duration_term (JITTER duration_term)? COLON? suite (ON_MISSED COLON? suite)?
    ;

within_stmt
    : WITHIN duration_term COLON? suite (FALLBACK COLON? suite)?
    ;

periodic_stmt
    : PERIODIC duration_term (PRIORITY test)? (JITTER duration_term)? COLON? suite
    ;

noalloc_stmt
    : NOALLOC COLON? suite
    ;

region_stmt
    : REGION identifier COLON? suite
    ;

duration_term
    : DURATION_LITERAL
    | test
    ;
