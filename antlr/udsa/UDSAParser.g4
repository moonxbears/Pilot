grammar UDSAParser;

import CoreParser, UDSALexer, CoreLexer;

// =============================================================================
// DOMAIN: Universal Domain-Specific Abstractions (UDSA) & Metaprogramming (Parser)
// =============================================================================

dsl_stmt
    : (DSL | DOMAIN) identifier (parameters | LPAR dsl_args? RPAR)? COLON? (suite | RAW_DSL_BLOCK)
    ;

dsl_args
    : test (COMMA test)*
    ;

unit_stmt
    : UNIT identifier (EQUAL unit_expr)? SEMI?
    ;

unit_expr
    : unit_term ((STAR | SLASH | DOUBLESTAR) unit_term)*
    ;

unit_term
    : identifier
    | INTEGER
    | LPAR unit_expr RPAR
    ;

macro_def
    : MACRO identifier generic_params? parameters COLON? suite
    ;

quote_stmt
    : QUOTE COLON? (suite | LBRACE test RBRACE)
    ;
