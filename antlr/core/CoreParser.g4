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
    | PUBLIC | PRIVATE | PROTECTED | INTERNAL
    | SYMBOL | REF | DYNAMIC | FN
    | STATIC | PARAMS | ARGS | KWARGS | NAMEOF | WHEN | UNDEFINED | PARTIAL | STEAL | CONTRACTS | DELEGATE
    | EXTERN | CSHARP | PYTHON | RUST | ASSEMBLY | MODULE | LIB
    | MACRO | DEFMACRO | QUOTE | QUASIQUOTE | SPLICE | UNQUOTE | UNQUOTE_SPLICING
    ;

stmt
    : compound_stmt
    | simple_stmt
    ;

simple_stmt
    : small_stmt (SEMI small_stmt)* SEMI?
    ;

small_stmt
    : import_stmt
    | let_stmt
    | var_stmt
    | const_stmt
    | ref_stmt
    | dynamic_stmt
    | type_alias_stmt
    | guard_stmt
    | defer_stmt
    | annassign_stmt
    | flow_stmt
    | global_stmt
    | assert_stmt
    | pass_stmt
    | expr_stmt
    ;

compound_stmt
    : labeled_stmt
    | func_def
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
    | extern_def
    | inline_foreign_block
    | macro_def
    | quote_stmt
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
    : REF? base_type QUESTION?
    ;

base_type
    : SYMBOL
    | DYNAMIC
    | (dotted_name | NONE | NULL | UNDEFINED) ( (LSQB type_arguments RSQB) | (LESS type_arguments GREATER) )?
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

// Accessibility & Declaration Modifiers
access_modifier
    : PUBLIC
    | PRIVATE
    | PROTECTED
    | INTERNAL
    ;

declaration_modifier
    : access_modifier
    | STATIC
    | PARTIAL
    ;

// Variables, Constants & Destructuring Declarations
binding_pattern
    : identifier
    | splice_expr
    | tuple_pattern
    | list_pattern
    | record_pattern
    ;

tuple_pattern
    : LPAR (binding_pattern_item (COMMA binding_pattern_item)* COMMA?)? RPAR
    ;

list_pattern
    : LSQB (binding_pattern_item (COMMA binding_pattern_item)* COMMA?)? RSQB
    ;

record_pattern
    : LBRACE (record_pattern_field (COMMA record_pattern_field)* COMMA?)? RBRACE
    ;

binding_pattern_item
    : (STAR | DOUBLESTAR)? binding_pattern (COLON type_spec)? (EQUAL test)?
    ;

record_pattern_field
    : identifier (COLON binding_pattern)? (EQUAL test)?
    | DOUBLESTAR identifier
    ;

let_stmt
    : declaration_modifier* LET binding_pattern type_annotation? (EQUAL test)?
    ;

var_stmt
    : declaration_modifier* VAR binding_pattern type_annotation? (EQUAL test)?
    ;

const_stmt
    : declaration_modifier* CONST binding_pattern type_annotation? EQUAL test
    ;

ref_stmt
    : declaration_modifier* REF binding_pattern type_annotation? (EQUAL test)?
    ;

dynamic_stmt
    : declaration_modifier* DYNAMIC binding_pattern (EQUAL test)?
    ;

type_alias_stmt
    : declaration_modifier* TYPE identifier generic_params? EQUAL type_spec
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
    : decorators? declaration_modifier* (PURE)? (DEF | FN) identifier generic_params? parameters (ARROW type_spec)? (COLON? suite | SEMI)?
    ;

parameters
    : LPAR (parameter (COMMA parameter)* COMMA?)? RPAR
    ;

parameter
    : (PARAMS | REF)? (STAR | DOUBLESTAR)? (identifier | splice_expr) type_annotation? (EQUAL test)?
    ;

class_def
    : decorators? declaration_modifier* CLASS identifier generic_params? (LPAR class_bases? RPAR)? COLON? suite
    ;

class_bases
    : type_spec (COMMA type_spec)*
    ;

struct_def
    : decorators? declaration_modifier* STRUCT identifier generic_params? COLON? (LBRACE struct_member* RBRACE | suite)
    ;

struct_member
    : declaration_modifier* (LET | VAR | CONST)? identifier type_annotation? (EQUAL test)? SEMI?
    ;

interface_def
    : decorators? declaration_modifier* (INTERFACE | PROTOCOL) identifier generic_params? COLON? suite
    ;

enum_def
    : decorators? declaration_modifier* ENUM identifier generic_params? COLON? enum_suite
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
labeled_stmt
    : identifier COLON (for_stmt | while_stmt | suite)
    ;

if_stmt
    : IF test COLON? suite ((ELIF | ELSE IF) test COLON? suite)* (ELSE COLON? suite)?
    ;

while_stmt
    : WHILE test COLON? suite (ELSE COLON? suite)?
    ;

for_stmt
    : FOR for_target IN test COLON? suite (ELSE COLON? suite)?
    ;

for_target
    : binding_pattern (COMMA binding_pattern)* COMMA?
    | exprlist
    ;

match_stmt
    : MATCH test COLON? match_suite
    ;

match_suite
    : LBRACE match_arm* RBRACE
    ;

match_arm
    : CASE pattern ((WHEN | IF) test)? (COLON | ARROW | FAT_ARROW)? suite
    ;

pattern
    : or_pattern
    ;

or_pattern
    : and_pattern ((OR | VBAR) and_pattern)*
    ;

and_pattern
    : not_pattern (AND not_pattern)*
    ;

not_pattern
    : NOT not_pattern
    | primary_pattern
    ;

primary_pattern
    : relational_pattern
    | collection_pattern
    | property_pattern
    | tuple_pattern_match
    | type_or_var_pattern
    | literal
    | symbol_literal
    | UNDERSCORE
    ;

relational_pattern
    : (LESS | LESSEQUAL | GREATER | GREATEREQUAL | EQEQUAL | NOTEQUAL) shift_expr
    ;

collection_pattern
    : LSQB (slice_pattern_element (COMMA slice_pattern_element)* COMMA?)? RSQB
    ;

slice_pattern_element
    : (DOTDOT | STAR) pattern?
    | pattern
    ;

property_pattern
    : LBRACE (property_pattern_member (COMMA property_pattern_member)* COMMA?)? RBRACE
    ;

property_pattern_member
    : identifier COLON pattern
    ;

tuple_pattern_match
    : LPAR (pattern (COMMA pattern)* COMMA?)? RPAR
    ;

type_or_var_pattern
    : (VAR | LET) identifier
    | dotted_name (LPAR (pattern (COMMA pattern)* COMMA?)? RPAR)? property_pattern? identifier?
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
    : BREAK identifier?
    | CONTINUE identifier?
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
    : foreign_import_stmt
    | import_name
    | import_from
    ;

foreign_import_stmt
    : IMPORT foreign_target (AS identifier)?
    | FROM foreign_target IMPORT (STAR | import_as_names | LPAR import_as_names RPAR)
    ;

foreign_target
    : (PYTHON | CSHARP | RUST | identifier) COLON (dotted_name | STRING)
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
    : lambda_expr
    | pipe_expr (IF pipe_expr ELSE test)?
    ;

lambda_expr
    : capture_list parameters (ARROW type_spec)? lambda_body
    | capture_list (ARROW type_spec)? lambda_body
    | parameters (ARROW type_spec)? (FAT_ARROW test | FAT_ARROW? LBRACE stmt* RBRACE)
    | identifier FAT_ARROW (test | LBRACE stmt* RBRACE)
    | (DEF | FN) parameters? ((ARROW | COLON) type_spec)? (FAT_ARROW test | COLON? LBRACE stmt* RBRACE)
    | LAMBDA lambda_params? (ARROW | COLON) test
    ;

lambda_body
    : FAT_ARROW test
    | FAT_ARROW? LBRACE stmt* RBRACE
    ;

capture_list
    : LSQB (capture_item (COMMA capture_item)* COMMA?)? RSQB
    ;

capture_item
    : (REF | AMPER)? identifier (EQUAL test)?
    | AMPER
    | EQUAL
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
    : range_expr ( (IS | IS NOT) pattern | comp_op range_expr )*
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
    | cascade_trailer
    | EXCLAMATION
    ;

cascade_trailer
    : (DOTDOT | SAFE_CASCADE) cascade_target
    ;

cascade_target
    : ( (LET | VAR)? binding_pattern (EQUAL | WALRUS) )? dotted_name (LSQB type_arguments RSQB)? LPAR arglist? RPAR (ARROW binding_pattern)?
    | dotted_name EQUAL test
    | LSQB subscriptlist RSQB EQUAL test
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
    : inline_foreign_block
    | quote_expr
    | quasiquote_expr
    | splice_expr
    | literal
    | macro_call
    | symbol_literal
    | nameof_expr
    | identifier
    | LPAR testlist_comp? RPAR
    | collection_expr
    | LBRACE dictorsetmaker? RBRACE
    ;

// Metaprogramming / Macros / Lisp Homoiconicity
macro_def
    : decorators? declaration_modifier* (MACRO | DEFMACRO) EXCLAMATION? identifier generic_params? parameters? (ARROW type_spec)? COLON? suite
    ;

quote_stmt
    : (QUOTE | QUASIQUOTE) COLON? (suite | LBRACE test RBRACE)
    ;

quote_expr
    : QUOTE COLON? (suite | LBRACE test RBRACE | LPAR test RPAR)
    ;

quasiquote_expr
    : (QUASIQUOTE | BACKQUOTE) COLON? (suite | LBRACE test RBRACE | LPAR test RPAR | identifier | literal)
    ;

splice_expr
    : DOLLAR AT (identifier | LPAR test RPAR)
    | (DOLLAR | SPLICE | UNQUOTE) (identifier | LPAR test RPAR)
    | UNQUOTE_SPLICING (identifier | LPAR test RPAR)
    ;

// Polyglot Interoperability (Python, C#, Rust FFI)
extern_def
    : EXTERN (STRING | identifier) ( (ASSEMBLY | MODULE | LIB) (STRING | identifier) )? COLON? (LBRACE extern_member* RBRACE | extern_member)
    ;

extern_member
    : decorators? declaration_modifier* (DEF | FN) identifier generic_params? parameters (ARROW type_spec)? (COLON? suite | SEMI)?
    | decorators? declaration_modifier* (CLASS | STRUCT | INTERFACE) identifier generic_params? (LPAR class_bases? RPAR)? COLON? (LBRACE extern_member* RBRACE | suite | SEMI)
    | small_stmt
    ;

inline_foreign_block
    : (PYTHON | CSHARP | RUST) (RAW_DSL_BLOCK | LBRACE stmt* RBRACE)
    ;

collection_expr
    : LSQB (collection_element (comp_for | (COMMA collection_element)* COMMA?))? RSQB
    ;

collection_element
    : (DOTDOT | STAR)? test
    ;

nameof_expr
    : NAMEOF LPAR (dotted_name | test) RPAR
    ;

macro_call
    : identifier EXCLAMATION (LPAR arglist? RPAR | LBRACE stmt* RBRACE | LSQB arglist? RSQB)
    ;

symbol_literal
    : SYMBOL (LPAR test? RPAR)?
    | COLON identifier
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
    | UNDEFINED
    ;

testlist_comp
    : test (comp_for | (COMMA test)* COMMA?)
    ;

dictorsetmaker
    : (dict_entry (comp_for | (COMMA dict_entry)* COMMA?))
    | (collection_element (comp_for | (COMMA collection_element)* COMMA?))
    ;

dict_entry
    : (DOUBLESTAR | DOTDOT) test
    | test COLON test
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
    : REF? (STAR | DOUBLESTAR)? test
    | identifier EQUAL test
    ;
