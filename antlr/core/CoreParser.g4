grammar CoreParser;

import CoreLexer;

// =============================================================================
// DOMAIN: Core Pythonic Syntax, Type Safety & Preconditions (Parser)
// =============================================================================

// Top-level program entry point
program
    : stmt* EOF
    ;

identifier
    : NAME
    | GUARD | DEFER | TYPE | PURE | MUT | IMMUT | LET | VAR | CONST
    | MATCH | CASE | EXCEPT
    ;

stmt
    : simple_stmt
    | compound_stmt
    ;

simple_stmt
    : small_stmt (SEMI small_stmt)* SEMI?
    ;

small_stmt
    : let_stmt
    | var_stmt
    | const_stmt
    | type_alias_stmt
    | guard_stmt
    | defer_stmt
    | annassign_stmt
    | expr_stmt
    | flow_stmt
    | import_stmt
    | global_stmt
    | assert_stmt
    | pass_stmt
    ;

compound_stmt
    : func_def
    | class_def
    | struct_def
    | interface_def
    | enum_def
    | guard_stmt
    | defer_stmt
    | if_stmt
    | while_stmt
    | for_stmt
    | match_stmt
    | try_stmt
    | with_stmt
    ;

suite
    : LBRACE stmt* RBRACE
    | simple_stmt
    ;

// Mandatory early-exit precondition: guard <condition> else { <exit_suite> }
guard_stmt
    : GUARD (LET pattern EQUAL test | test) ELSE COLON? suite
    ;

// Schedule cleanup code to execute when exiting the containing scope/function
defer_stmt
    : DEFER (COLON? suite | small_stmt | test)
    ;

// Type System & Annotations
type_annotation
    : COLON type_spec
    ;

return_type
    : ARROW type_spec
    ;

type_spec
    : union_type
    ;

union_type
    : intersection_type (VBAR intersection_type)*
    ;

intersection_type
    : primary_type (AMPER primary_type)*
    ;

primary_type
    : base_type QUESTION?
    ;

base_type
    : (dotted_name | NONE | NULL) (LSQB type_arguments RSQB)?
    | LPAR type_spec (COMMA type_spec)* RPAR
    | LPAR (type_spec (COMMA type_spec)*)? RPAR ARROW type_spec
    | LSQB type_spec (SEMI test)? RSQB
    | LBRACE type_spec COLON type_spec RBRACE
    ;

type_arguments
    : type_spec (COMMA type_spec)*
    ;

generic_params
    : LSQB generic_param (COMMA generic_param)* RSQB
    ;

generic_param
    : identifier (COLON type_spec)?
    ;

// Variables & Constants
let_stmt
    : LET identifier type_annotation? (EQUAL test)?
    ;

var_stmt
    : VAR identifier type_annotation? (EQUAL test)?
    ;

const_stmt
    : CONST identifier type_annotation? EQUAL test
    ;

type_alias_stmt
    : TYPE identifier generic_params? EQUAL type_spec
    ;

annassign_stmt
    : test COLON type_spec (EQUAL test)?
    ;

expr_stmt
    : testlist (augassign test | (EQUAL | WALRUS) test)*
    ;

augassign
    : PLUSEQUAL
    | MINEQUAL
    | STAREQUAL
    | SLASHEQUAL
    | PERCENTEQUAL
    | AMPEREQUAL
    | VBAREQUAL
    | CIRCUMFLEXEQUAL
    | LEFTSHIFTEQUAL
    | RIGHTSHIFTEQUAL
    | DOUBLESTAREQUAL
    | DOUBLESLASHEQUAL
    | COALESCE_EQUAL
    ;

// Functions & Object Model
func_def
    : decorators? (PURE)? DEF identifier generic_params? parameters (ARROW type_spec)? (COLON? suite | SEMI)?
    ;

parameters
    : LPAR (parameter (COMMA parameter)* COMMA?)? RPAR
    ;

parameter
    : (STAR | DOUBLESTAR)? identifier type_annotation? (EQUAL test)?
    ;

class_def
    : decorators? CLASS identifier generic_params? (LPAR class_bases? RPAR)? COLON? suite
    ;

class_bases
    : type_spec (COMMA type_spec)*
    ;

struct_def
    : decorators? STRUCT identifier generic_params? COLON? (LBRACE struct_member* RBRACE | suite)
    ;

struct_member
    : (LET | VAR)? identifier type_annotation (EQUAL test)? SEMI?
    ;

interface_def
    : decorators? (INTERFACE | PROTOCOL) identifier generic_params? COLON? suite
    ;

enum_def
    : decorators? ENUM identifier generic_params? COLON? enum_suite
    ;

enum_suite
    : LBRACE enum_member (COMMA enum_member)* COMMA? RBRACE
    ;

enum_member
    : identifier (LPAR (type_spec | parameter) (COMMA (type_spec | parameter))* COMMA? RPAR)? (EQUAL test)? SEMI?
    ;

decorators
    : decorator+
    ;

decorator
    : AT dotted_name (LPAR arglist? RPAR)?
    ;

// Control Flow
if_stmt
    : IF test COLON? suite (ELIF test COLON? suite)* (ELSE COLON? suite)?
    ;

while_stmt
    : WHILE test COLON? suite (ELSE COLON? suite)?
    ;

for_stmt
    : FOR for_target IN test COLON? suite (ELSE COLON? suite)?
    ;

for_target
    : exprlist
    ;

match_stmt
    : MATCH test COLON? match_suite
    ;

match_suite
    : LBRACE match_arm* RBRACE
    ;

match_arm
    : CASE pattern (IF test)? (COLON | ARROW)? suite
    ;

pattern
    : pattern_term (VBAR pattern_term)*
    ;

pattern_term
    : literal
    | dotted_name (LPAR (pattern (COMMA pattern)*)? RPAR)?
    | LSQB (pattern (COMMA pattern)*)? RSQB
    | LPAR (pattern (COMMA pattern)*)? RPAR
    | STAR identifier
    | UNDERSCORE
    ;

try_stmt
    : TRY COLON? suite except_clause* (ELSE COLON? suite)? (FINALLY COLON? suite)?
    ;

except_clause
    : EXCEPT (type_spec (AS identifier)?)? COLON? suite
    ;

with_stmt
    : WITH with_item (COMMA with_item)* COLON? suite
    ;

with_item
    : test (AS identifier)?
    ;

flow_stmt
    : BREAK
    | CONTINUE
    | RETURN testlist?
    | YIELD testlist?
    | RAISE (test (FROM test)?)?
    ;

pass_stmt
    : PASS
    ;

assert_stmt
    : ASSERT test (COMMA test)?
    ;

global_stmt
    : GLOBAL identifier (COMMA identifier)*
    ;

import_stmt
    : import_name
    | import_from
    ;

import_name
    : IMPORT dotted_as_names
    ;

import_from
    : FROM (DOT* dotted_name | DOT+) IMPORT (STAR | import_as_names | LPAR import_as_names RPAR)
    ;

import_as_name
    : identifier (AS identifier)?
    ;

dotted_as_name
    : dotted_name (AS identifier)?
    ;

import_as_names
    : import_as_name (COMMA import_as_name)* COMMA?
    ;

dotted_as_names
    : dotted_as_name (COMMA dotted_as_name)*
    ;

dotted_name
    : identifier (DOT identifier)*
    ;

// Expressions & Operators
testlist
    : test (COMMA test)* COMMA?
    ;

exprlist
    : test (COMMA test)* COMMA?
    ;

test
    : pipe_expr (IF pipe_expr ELSE test)?
    | lambdef
    ;

lambdef
    : LAMBDA lambda_params? (ARROW | COLON) test
    ;

lambda_params
    : parameters
    | lambda_param (COMMA lambda_param)*
    ;

lambda_param
    : (STAR | DOUBLESTAR)? identifier type_annotation? (EQUAL test)?
    ;

pipe_expr
    : coalesce_expr ((PIPE_FORWARD | PIPE_BACKWARD) coalesce_expr)*
    ;

coalesce_expr
    : or_test (COALESCE or_test)*
    ;

or_test
    : and_test (OR and_test)*
    ;

and_test
    : not_test (AND not_test)*
    ;

not_test
    : NOT not_test
    | comparison
    ;

comparison
    : range_expr (comp_op range_expr)*
    ;

comp_op
    : EQEQUAL
    | NOTEQUAL
    | LESS
    | LESSEQUAL
    | GREATER
    | GREATEREQUAL
    | IN
    | NOT IN
    | IS
    | IS NOT
    ;

range_expr
    : bitor_expr ((DOTDOT | DOTDOT_EQUAL) bitor_expr)?
    ;

bitor_expr
    : bitxor_expr (VBAR bitxor_expr)*
    ;

bitxor_expr
    : bitand_expr (CIRCUMFLEX bitand_expr)*
    ;

bitand_expr
    : shift_expr (AMPER shift_expr)*
    ;

shift_expr
    : arith_expr ((LEFTSHIFT | RIGHTSHIFT) arith_expr)*
    ;

arith_expr
    : term ((PLUS | MINUS) term)*
    ;

term
    : factor ((STAR | SLASH | DOUBLESLASH | PERCENT) factor)*
    ;

factor
    : (PLUS | MINUS | TILDE) factor
    | power
    ;

power
    : atom_expr (DOUBLESTAR factor)?
    ;

atom_expr
    : atom trailer*
    ;

trailer
    : LPAR arglist? RPAR
    | LSQB subscriptlist RSQB
    | DOT identifier
    | SAFE_NAV identifier
    | EXCLAMATION
    ;

subscriptlist
    : subscript (COMMA subscript)* COMMA?
    ;

subscript
    : DOTDOT
    | test DOTDOT test?
    | DOTDOT test
    | test DOTDOT_EQUAL test
    | test
    ;

atom
    : literal
    | identifier
    | LPAR testlist_comp? RPAR
    | LSQB testlist_comp? RSQB
    | LBRACE dictorsetmaker? RBRACE
    ;

literal
    : INTEGER
    | FLOAT_NUMBER
    | HEX_INTEGER
    | BIN_INTEGER
    | OCT_INTEGER
    | STRING+
    | TRIPLE_STRING+
    | FSTRING+
    | TRUE
    | FALSE
    | NONE
    | NULL
    ;

testlist_comp
    : test (comp_for | (COMMA test)* COMMA?)
    ;

dictorsetmaker
    : (test COLON test (comp_for | (COMMA test COLON test)* COMMA?))
    | (test (comp_for | (COMMA test)* COMMA?))
    ;

comp_for
    : FOR exprlist IN test comp_iter?
    ;

comp_iter
    : comp_for
    | comp_if
    ;

comp_if
    : IF test comp_iter?
    ;

arglist
    : argument (COMMA argument)* COMMA?
    ;

argument
    : (STAR | DOUBLESTAR)? test
    | identifier EQUAL test
    ;
