lexer grammar RealtimeLexer;

// =============================================================================
// DOMAIN: Soft Real-Time vs Hard Real-Time Systems (Lexer)
// =============================================================================

REALTIME    : 'realtime';
DEADLINE    : 'deadline';
ON_MISSED   : 'on_missed';
WITHIN      : 'within';
FALLBACK    : 'fallback';
PERIOD      : 'period';
PERIODIC    : 'periodic';
PRIORITY    : 'priority';
JITTER      : 'jitter';
BOUNDED     : 'bounded';
NOALLOC     : 'noalloc';
STACKALLOC  : 'stackalloc';
REGION      : 'region';
WCET        : 'wcet';
HARD_RT     : 'hard_rt';
SOFT_RT     : 'soft_rt';

DURATION_LITERAL
    : [0-9]+ ('.' [0-9]+)? ('ns' | 'us' | 'ms' | 's' | 'min' | 'h')
    ;

FREQUENCY_LITERAL
    : [0-9]+ ('.' [0-9]+)? ('Hz' | 'kHz' | 'MHz' | 'GHz')
    ;

MEMORY_LITERAL
    : [0-9]+ ('B' | 'KB' | 'MB' | 'GB' | 'TB')
    ;
