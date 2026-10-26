lexer grammar UDSALexer;

// =============================================================================
// DOMAIN: Universal Domain-Specific Abstractions (UDSA) & Metaprogramming (Lexer)
// =============================================================================

DSL         : 'dsl';
DOMAIN      : 'domain';
UNIT        : 'unit';
MACRO       : 'macro';
QUOTE       : 'quote';
SPLICE      : 'splice';

RAW_DSL_BLOCK
    : '```' [a-zA-Z0-9_]* ~[\r\n]* ('\r'? '\n') .*? '```'
    ;

DOLLAR      : '$';
BACKQUOTE   : '`';
