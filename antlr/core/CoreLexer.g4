lexer grammar CoreLexer;

// =============================================================================
// DOMAIN: Core Pythonic Syntax, Type Safety & Preconditions (Lexer)
// =============================================================================

// Declaration Keywords
DEF         : 'def' | 'fn';
CLASS       : 'class';
STRUCT      : 'struct';
INTERFACE   : 'interface';
PROTOCOL    : 'protocol';
ENUM        : 'enum';
LET         : 'let';
VAR         : 'var';
CONST       : 'const';
TYPE        : 'type';
PURE        : 'pure';
MUT         : 'mut';
IMMUT       : 'immut';

// Robust Control Flow & Precondition Keywords
GUARD       : 'guard';
DEFER       : 'defer';
IF          : 'if';
ELIF        : 'elif';
ELSE        : 'else';
WHILE       : 'while';
FOR         : 'for';
IN          : 'in';
RETURN      : 'return';
YIELD       : 'yield';
BREAK       : 'break';
CONTINUE    : 'continue';
PASS        : 'pass';
RAISE       : 'raise';
TRY         : 'try';
EXCEPT      : 'except';
FINALLY     : 'finally';
WITH        : 'with';
AS          : 'as';
ASSERT      : 'assert';
IMPORT      : 'import';
FROM        : 'from';
GLOBAL      : 'global';
LAMBDA      : 'lambda';
MATCH       : 'match';
CASE        : 'case';
AND         : 'and';
OR          : 'or';
NOT         : 'not';
IS          : 'is';

// Built-in Literals
TRUE        : 'True' | 'true';
FALSE       : 'False' | 'false';
NONE        : 'None' | 'none';
NULL        : 'null';

// Operators & Delimiters
ARROW            : '->';
FAT_ARROW        : '=>';
PIPE_FORWARD     : '|>';
PIPE_BACKWARD    : '<|';
SAFE_NAV         : '?.';
COALESCE_EQUAL   : '??=';
COALESCE         : '??';
DOTDOT_EQUAL     : '..=';
DOTDOT           : '..';
DOUBLESTAR       : '**';
DOUBLESLASH      : '//';
EQEQUAL          : '==';
NOTEQUAL         : '!=' | '<>';
LESSEQUAL        : '<=';
GREATEREQUAL     : '>=';
LEFTSHIFT        : '<<';
RIGHTSHIFT       : '>>';
PLUSEQUAL        : '+=';
MINEQUAL         : '-=';
STAREQUAL        : '*=';
SLASHEQUAL       : '/=';
PERCENTEQUAL     : '%=';
AMPEREQUAL       : '&=';
VBAREQUAL        : '|=';
CIRCUMFLEXEQUAL  : '^=';
LEFTSHIFTEQUAL   : '<<=';
RIGHTSHIFTEQUAL  : '>>=';
DOUBLESTAREQUAL  : '**=';
DOUBLESLASHEQUAL : '//=';
WALRUS           : ':=';

LPAR        : '(';
RPAR        : ')';
LSQB        : '[';
RSQB        : ']';
LBRACE      : '{';
RBRACE      : '}';
COLON       : ':';
SEMI        : ';';
COMMA       : ',';
DOT         : '.';
AT          : '@';
PLUS        : '+';
MINUS       : '-';
STAR        : '*';
SLASH       : '/';
PERCENT     : '%';
VBAR        : '|';
AMPER       : '&';
CIRCUMFLEX  : '^';
TILDE       : '~';
LESS        : '<';
GREATER     : '>';
EQUAL       : '=';
QUESTION    : '?';
EXCLAMATION : '!';
UNDERSCORE  : '_';

// Numeric Literals
FLOAT_NUMBER
    : [0-9]+ '.' [0-9]+ ([eE] [+-]? [0-9]+)?
    | [0-9]+ [eE] [+-]? [0-9]+
    ;

HEX_INTEGER
    : '0' [xX] [0-9a-fA-F_]+
    ;

BIN_INTEGER
    : '0' [bB] [01_]+
    ;

OCT_INTEGER
    : '0' [oO] [0-7_]+
    ;

INTEGER
    : [0-9] [0-9_]*
    ;

// String Literals
TRIPLE_STRING
    : '"""' .*? '"""'
    | '\'\'\'' .*? '\'\'\''
    ;

FSTRING
    : [fF] ('"""' .*? '"""' | '\'\'\'' .*? '\'\'\'' | '"' (~["\\\r\n] | '\\' .)* '"' | '\'' (~['\\\r\n] | '\\' .)* '\'')
    ;

STRING
    : '"' (~["\\\r\n] | '\\' .)* '"'
    | '\'' (~['\\\r\n] | '\\' .)* '\''
    ;

// Identifiers
NAME
    : [a-zA-Z_][a-zA-Z0-9_]*
    ;

// Whitespace & Comments
WS
    : [ \t\f\r\n]+ -> channel(HIDDEN)
    ;

COMMENT
    : ('#' | '//') ~[\r\n]* -> channel(HIDDEN)
    ;

BLOCK_COMMENT
    : '/*' .*? '*/' -> channel(HIDDEN)
    ;

LINE_JOINING
    : '\\' '\r'? '\n' -> channel(HIDDEN)
    ;

ERRORTOKEN
    : .
    ;
