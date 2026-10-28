grammar Pilot;

// =============================================================================
// Language: PILOT
// Type-Safe Pythonic Syntax with:
//  - In-Memory & Postgres-like Generic Database Integration
//  - Robust Preconditions (guard) & Exit Scheduling (defer)
//  - C#-like Robustness (Properties, Operator Overloading, Extensions, Where-Constraints)
//  - Asynchronous & Multi-Threaded Concurrency
//  - Soft Real-Time vs. Hard Real-Time Trade-offs
//  - Universal Domain-Specific Abstractions (UDSA)
// =============================================================================

// Top-level program entry point
program
    : stmt* EOF
    ;

// =============================================================================
// Identifiers & Contextual Keyword Support
// =============================================================================

identifier
    : NAME
    | REALTIME | HARD_RT | SOFT_RT | WCET | DEFAULT | TIMEOUT
    | INDEX | INDEXED | LOOKUP | TABLE | SCHEMA | UNIT | DOMAIN | DSL
    | ORDER | GROUP | BY | FROM | SELECT | WHERE | JOIN
    | INNER | LEFT | RIGHT | FULL | CROSS | OUTER | ON | HAVING | ASC | DESC | ASCENDING | DESCENDING | LIMIT | OFFSET | INTO | DISTINCT | UNION | ALL | INTERSECT | DIFFERENCE
    | REGION | NOALLOC | STACKALLOC | PERIOD | PERIODIC
    | DEADLINE | WITHIN | FALLBACK | PRIORITY | JITTER | BOUNDED
    | SPAWN | PARALLEL | ATOMIC | SYNC | LOCK | ISOLATE
    | CHAN | CHANNEL | ASYNC | AWAIT | ACTOR | MACRO | QUOTE | SPLICE
    | DEFMACRO | QUASIQUOTE | UNQUOTE | UNQUOTE_SPLICING
    | MATCH | CASE | TYPE | PURE | MUT | IMMUT | LET | VAR | CONST
    | GUARD | DEFER | INSERT | UPDATE | DELETE | VALUES | SET_KW
    | RETURNING | CONFLICT | DO | NOTHING | OVER | PARTITION | WINDOW
    | TRANSACTION | COMMIT | ROLLBACK | SAVEPOINT | TO | CONNECT | DATABASE | MODEL
    | PROPERTY | GET | OPERATOR | EXTENSION | EXTEND
    | EXCEPT | CATCH
    | PUBLIC | PRIVATE | PROTECTED | INTERNAL
    | SYMBOL | REF | DYNAMIC | FN
    | STATIC | PARAMS | ARGS | KWARGS | NAMEOF | WHEN | UNDEFINED | PARTIAL | STEAL | CONTRACTS | DELEGATE
    | EXTERN | CSHARP | PYTHON | RUST | ASSEMBLY | MODULE | LIB | STATE
    | CONTEXT | REQUIRES | ENSURES
    | JS | JAVASCRIPT | JAVA | ASM | CPP | C_KW | LUA | SQL_KW | POSTGRES | POSTGRESQL | MYSQL
    | EACH | REPEAT | ADAPTER | CONVERTER | SOME | THEN
    ;

// =============================================================================
// Statements
// =============================================================================

stmt
    : compound_stmt
    | simple_stmt
    ;

simple_stmt
    : small_stmt ({false}? SEMI small_stmt)* SEMI?
    ;

small_stmt
    : import_stmt
    | let_stmt
    | var_stmt
    | const_stmt
    | ref_stmt
    | dynamic_stmt
    | type_alias_stmt
    | unit_stmt
    | guard_stmt
    | defer_stmt
    | commit_stmt
    | rollback_stmt
    | database_stmt
    | savepoint_stmt
    | annassign_stmt
    | channel_send_stmt
    | flow_stmt
    | global_stmt
    | assert_stmt
    | pass_stmt
    | steal_stmt
    | delegate_stmt
    | contract_forward_stmt
    | pattern_assign_stmt
    | expr_stmt
    ;

compound_stmt
    : labeled_stmt
    | func_def
    | class_def
    | state_def
    | struct_def
    | record_def
    | interface_def
    | enum_def
    | actor_def
    | model_def
    | property_def
    | operator_def
    | extension_def
    | adapter_def
    | macro_def
    | extern_def
    | inline_foreign_block
    | quote_stmt
    | guard_stmt
    | defer_stmt
    | if_stmt
    | while_stmt
    | for_stmt
    | match_stmt
    | try_stmt
    | with_stmt
    // Database compound blocks
    | transaction_stmt
    | savepoint_block
    | database_def
    // Real-time compound blocks
    | deadline_stmt
    | within_stmt
    | periodic_stmt
    | noalloc_stmt
    | region_stmt
    // Concurrency compound blocks
    | spawn_stmt
    | atomic_stmt
    | sync_stmt
    | isolate_stmt
    | select_stmt
    // Universal Domain-Specific Abstractions
    | dsl_stmt
    ;

// =============================================================================
// Suites & Blocks (Delimited by {} Braces)
// =============================================================================

suite
    : LBRACE stmt* RBRACE
    | simple_stmt
    ;

// =============================================================================
// Robustness: Guard (Precondition) and Defer (Exit Cleanup)
// =============================================================================

// Mandatory early-exit precondition: guard <condition> else { <exit_suite> } or guard let x = expr catch err { <exit_suite> }
guard_stmt
    : GUARD ((LET | VAR | SOME) pattern EQUAL test pattern_guard? | SOME? test pattern_guard?) ((ELSE | CATCH identifier?) COLON? suite | suite)?
    ;

pattern_guard
    : WHEN test
    ;

// Schedule cleanup code to execute when exiting the containing scope/function
defer_stmt
    : DEFER (COLON? suite | small_stmt | test)
    ;

// =============================================================================
// Type System & Annotations (Type Safety)
// =============================================================================

type_annotation
    : COLON type_spec
    ;

return_type
    : ARROW type_spec
    ;

type_spec
    : union_type (EXCLAMATION error_union_type?)?
    ;

error_union_type
    : error_primary_type (VBAR error_primary_type)*
    ;

error_primary_type
    : SYMBOL
    | DYNAMIC
    | (dotted_name | NONE | NULL | UNDEFINED) ( (LSQB type_arguments RSQB) | (LESS type_arguments GREATER) )? (LSQB RSQB | QUESTION)*
    | LPAR type_spec (COMMA type_spec)* RPAR
    ;

union_type
    : intersection_type (VBAR intersection_type)*
    ;

intersection_type
    : primary_type (AMPER primary_type)*
    ;

primary_type
    : REF? base_type (LSQB RSQB | QUESTION | COALESCE | STAR)* (LESS unit_expr GREATER)?
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

// C#-style generic type constraints: where T: IFoo, U: IBar
where_type_clause
    : WHERE where_type_item (COMMA where_type_item)*
    ;

where_type_item
    : identifier COLON type_spec
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

// =============================================================================
// Variable, Constant & Destructuring Declarations
// =============================================================================

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
    : (STAR | DOUBLESTAR | DOTDOT)? binding_pattern (COLON type_spec)? (EQUAL test)?
    ;

record_pattern_field
    : (identifier | STRING) (COLON binding_pattern)? (EQUAL test)?
    | DOUBLESTAR identifier
    ;

pattern_assign_stmt
    : (tuple_pattern | list_pattern | record_pattern) EQUAL test
    ;

let_stmt
    : declaration_modifier* LET binding_pattern type_annotation? (EQUAL test)?
    ;

var_stmt
    : declaration_modifier* VAR binding_pattern type_annotation? (EQUAL test)? index_clause?
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
    : decorators? declaration_modifier* TYPE identifier generic_params? EQUAL adt_sum_body
    | decorators? declaration_modifier* TYPE identifier generic_params? EQUAL type_spec
    ;

adt_sum_body
    : VBAR adt_variant (VBAR adt_variant)*
    | adt_variant_payload (VBAR adt_variant)*
    | identifier (VBAR identifier)* VBAR adt_variant_payload (VBAR adt_variant)*
    ;

adt_variant
    : identifier (LPAR adt_field_list? RPAR | LBRACE struct_member* RBRACE)?
    ;

adt_variant_payload
    : identifier (LPAR adt_field_list? RPAR | LBRACE struct_member* RBRACE)
    ;

adt_field_list
    : adt_field (COMMA adt_field)* COMMA?
    ;

adt_field
    : identifier COLON type_spec (EQUAL test)?
    | type_spec
    ;

annassign_stmt
    : test COLON type_spec (EQUAL test)?
    ;

expr_stmt
    : testlist (augassign test | (EQUAL | WALRUS) test)*
    ;

channel_send_stmt
    : test ARROW_LEFT test
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

// =============================================================================
// Functions, Classes, Structs, Interfaces, Enums, Actors
// =============================================================================

func_def
    : decorators? declaration_modifier* (PURE | ASYNC)? (DEF | FN) identifier generic_params? parameters (ARROW type_spec)? where_type_clause? COLON? contract_clause* (COLON | FAT_ARROW)? (suite | SEMI)?
    ;

contract_clause
    : REQUIRES test
    | ENSURES test
    ;

parameters
    : LPAR (parameter (COMMA parameter)* COMMA?)? RPAR
    ;

parameter
    : CONTEXT? (EACH | REPEAT)? (PARAMS | REF)? (STAR | DOUBLESTAR)? (identifier | splice_expr) type_annotation? (EQUAL test)?
    ;

class_def
    : decorators? declaration_modifier* CLASS identifier generic_params? EQUAL test SEMI?
    | decorators? declaration_modifier* CLASS identifier generic_params? (LPAR class_bases? RPAR)? where_type_clause? COLON? suite
    ;

state_def
    : STATE identifier generic_params? COLON? suite
    ;

class_bases
    : type_spec (COMMA type_spec)*
    ;

// Structs: Unboxed value-types critical for Hard Real-Time & zero-GC allocations
struct_def
    : decorators? declaration_modifier* STRUCT identifier generic_params? where_type_clause? COLON? (LBRACE struct_member* RBRACE | suite)
    ;

// Records: Immutable value-semantic product types
record_def
    : decorators? declaration_modifier* RECORD identifier generic_params? LPAR adt_field_list? RPAR where_type_clause? (COLON? LBRACE stmt* RBRACE | SEMI)?
    | decorators? declaration_modifier* RECORD identifier generic_params? where_type_clause? COLON? LBRACE struct_member* RBRACE SEMI?
    ;

struct_member
    : declaration_modifier* (LET | VAR | CONST)? identifier type_annotation? (EQUAL test)? SEMI?
    ;

// Interfaces / Protocols: Strong structural and nominal typing
interface_def
    : decorators? declaration_modifier* (INTERFACE | PROTOCOL) identifier generic_params? where_type_clause? COLON? suite
    ;

// Algebraic Data Types & Enums
enum_def
    : decorators? declaration_modifier* ENUM identifier generic_params? COLON? enum_suite
    ;

enum_suite
    : LBRACE VBAR? enum_member ((COMMA | VBAR) enum_member)* COMMA? RBRACE
    ;

enum_member
    : identifier (LPAR adt_field_list? RPAR | LBRACE struct_member* RBRACE)? (EQUAL test)? SEMI?
    ;

// Actor concurrency model
actor_def
    : decorators? declaration_modifier* ACTOR identifier generic_params? COLON? suite
    ;

// Database Models / Schemas / Tables
model_def
    : decorators? declaration_modifier* (MODEL | TABLE | SCHEMA) identifier generic_params? (LPAR class_bases? RPAR)? COLON? suite
    ;

// =============================================================================
// C#-style Customization: Properties, Operators & Extensions
// =============================================================================

// Properties with get / set accessors
property_def
    : decorators? declaration_modifier* PROPERTY identifier type_annotation? (FAT_ARROW suite | COLON? property_body)
    ;

property_body
    : LBRACE property_accessor* RBRACE
    ;

property_accessor
    : get_accessor
    | set_accessor
    ;

get_accessor
    : GET (COLON | FAT_ARROW)? (suite | SEMI)?
    ;

set_accessor
    : SET_KW (LPAR identifier RPAR)? (COLON | FAT_ARROW)? (suite | SEMI)?
    ;

// Operator Overloading
operator_def
    : decorators? declaration_modifier* OPERATOR overloadable_op parameters (ARROW type_spec)? COLON? suite
    ;

overloadable_op
    : PLUS | MINUS | STAR | SLASH | PERCENT | DOUBLESTAR | DOUBLESLASH
    | EQEQUAL | NOTEQUAL | LESS | LESSEQUAL | GREATER | GREATEREQUAL
    | AMPER | VBAR | CIRCUMFLEX | TILDE | LEFTSHIFT | RIGHTSHIFT
    | LSQB RSQB EQUAL?
    ;

// Extension methods / classes: extension for Type: ... or extension Name on Type: ... or extend Type { ... }
extension_def
    : decorators? declaration_modifier* EXTENSION identifier? generic_params? (FOR | ON) type_spec where_type_clause? COLON? suite
    | decorators? declaration_modifier* EXTEND generic_params? type_spec where_type_clause? COLON? suite
    ;

// Type Adapter Definition: adapter FromType -> ToType: ... or adapter Name: ...
adapter_def
    : decorators? declaration_modifier* ADAPTER (type_spec (ARROW | TO) type_spec | identifier) COLON? suite
    ;

// Compile-Time Metaprogramming / Macros (Lisp Homoiconicity)
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

foreign_language
    : PYTHON | CSHARP | RUST | JS | JAVASCRIPT | JAVA | ASM | ASSEMBLY | CPP | C_KW | LUA | SQL_KW | POSTGRES | POSTGRESQL | MYSQL
    ;

inline_foreign_block
    : foreign_language (RAW_DSL_BLOCK | LBRACE stmt* RBRACE | TRIPLE_STRING | STRING)
    ;

// Decorators
decorators
    : decorator+
    ;

decorator
    : AT dotted_name (LPAR arglist? RPAR)?
    ;

// =============================================================================
// Control Flow Statements
// =============================================================================

labeled_stmt
    : identifier COLON (for_stmt | while_stmt | block)
    ;

block
    : LBRACE stmt* RBRACE
    ;

if_stmt
    : IF (((LET | VAR | SOME) pattern EQUAL) | SOME)? test pattern_guard? COLON? suite ((ELIF | ELSE IF) (((LET | VAR | SOME) pattern EQUAL) | SOME)? test pattern_guard? COLON? suite)* (ELSE COLON? suite)?
    ;

while_stmt
    : WHILE (((LET | VAR | SOME) pattern EQUAL) | SOME)? test pattern_guard? (BOUNDED test)? COLON? suite (ELSE COLON? suite)?
    ;

for_stmt
    : (ASYNC | PARALLEL)? FOR (LPAR for_target IN test RPAR | for_target IN test) (BOUNDED test)? COLON? suite ((THEN | ELSE) COLON? suite)?
    ;

for_target
    : binding_pattern (COMMA binding_pattern)* COMMA?
    | exprlist
    ;

match_stmt
    : MATCH test COLON? match_suite
    | MATCH COLON? match_suite
    ;

match_suite
    : LBRACE match_arm* RBRACE
    ;

match_arm
    : CASE pattern ((WHEN | IF) test)? (COLON | ARROW | FAT_ARROW)? suite COMMA?
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
    | (VAR | LET)? identifier AT not_pattern
    | primary_pattern
    ;

primary_pattern
    : relational_pattern
    | range_pattern
    | signed_number_pattern
    | collection_pattern
    | property_pattern
    | tuple_pattern_match
    | keyword_pattern
    | type_or_var_pattern
    | literal
    | UNDERSCORE
    ;

range_pattern
    : range_bound (DOTDOT | DOTDOT_EQUAL | ELLIPSIS | DOTDOT_LESS) range_bound
    ;

range_bound
    : MINUS? (INTEGER | FLOAT_NUMBER | HEX_INTEGER | BIN_INTEGER | OCT_INTEGER)
    | STRING
    ;

signed_number_pattern
    : MINUS (INTEGER | FLOAT_NUMBER | HEX_INTEGER | BIN_INTEGER | OCT_INTEGER)
    ;

keyword_pattern
    : symbol_literal (LPAR (pattern_arg (COMMA pattern_arg)* COMMA?)? RPAR)?
    ;

pattern_arg
    : identifier COLON pattern
    | pattern
    ;

relational_pattern
    : (LESS | LESSEQUAL | GREATER | GREATEREQUAL | EQEQUAL | NOTEQUAL) shift_expr
    ;

collection_pattern
    : LSQB (slice_pattern_element (COMMA slice_pattern_element)* COMMA?)? RSQB
    ;

slice_pattern_element
    : (DOTDOT | ELLIPSIS | STAR) pattern?
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
    | dotted_name generic_pattern_suffix? (LPAR (pattern_arg (COMMA pattern_arg)* COMMA?)? RPAR)? property_pattern? identifier?
    ;

generic_pattern_suffix
    : LSQB type_arguments RSQB (DOT identifier)?
    ;

try_stmt
    : TRY COLON? suite except_clause* (ELSE COLON? suite)? (FINALLY COLON? suite)?
    ;

except_clause
    : EXCEPT ((type_spec (AS identifier)?) | (AS identifier))? COLON? suite
    ;

with_stmt
    : ASYNC? WITH with_item (COMMA with_item)* COLON? suite
    ;

with_item
    : EXTENSION identifier
    | CONTEXT LPAR with_context_binding (COMMA with_context_binding)* COMMA? RPAR
    | CONTEXT with_context_binding
    | test (AS identifier)?
    ;

with_context_binding
    : identifier (EQUAL | COLON) test
    ;

flow_stmt
    : BREAK (test | identifier)?
    | CONTINUE (range_expr (COMMA ELLIPSIS)? | identifier)?
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

steal_stmt
    : STEAL FROM? dotted_name (DOT identifier | LSQB identifier (COMMA identifier)* RSQB)?
    ;

delegate_stmt
    : DELEGATE type_spec TO test
    ;

contract_forward_stmt
    : CONTRACTS FOR? type_spec (EQUAL | COLON) test
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
    : (foreign_language | identifier) COLON (dotted_name | STRING)
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

// =============================================================================
// Soft Real-Time vs. Hard Real-Time Statements
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

// =============================================================================
// Asynchronous and Multi-Threaded Concurrency Statements
// =============================================================================

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
    | TIMEOUT duration_term (COLON | ARROW)? suite
    | DEFAULT (COLON | ARROW)? suite
    ;

select_case_target
    : identifier EQUAL ARROW_LEFT test   // msg = <-ch
    | test ARROW_LEFT test               // ch <- msg
    | ARROW_LEFT test                    // <-ch
    ;

// =============================================================================
// Universal Domain-Specific Abstractions (UDSA)
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

// =============================================================================
// Native & Postgres-like Generic Database Integration
// =============================================================================

// Generic Database definition / connection scope
database_stmt
    : DATABASE identifier type_annotation? EQUAL test
    ;

database_def
    : DATABASE identifier (LPAR testlist? RPAR)? COLON? suite
    ;

// Transaction & Savepoint control
transaction_stmt
    : TRANSACTION (LPAR test RPAR)? COLON? suite
    ;

savepoint_stmt
    : SAVEPOINT identifier
    ;

savepoint_block
    : SAVEPOINT identifier COLON? suite
    ;

commit_stmt
    : COMMIT (TRANSACTION)?
    ;

rollback_stmt
    : ROLLBACK (TO SAVEPOINT? identifier | TRANSACTION)?
    ;

// Postgres-like INSERT: insert into db.users (name, email) values (...) on conflict (...) do update returning id
insert_stmt
    : INSERT INTO dotted_name (LPAR identifier (COMMA identifier)* COMMA? RPAR)?
      (VALUES insert_values_group (COMMA insert_values_group)* | query_expr | sql_query_expr)
      on_conflict_clause?
      returning_clause?
    ;

insert_values_group
    : LPAR testlist? RPAR
    ;

on_conflict_clause
    : ON CONFLICT (LPAR identifier (COMMA identifier)* RPAR)? DO (NOTHING | UPDATE SET_KW set_assignment_list)
    ;

// Postgres-like UPDATE: update db.users set age = 30 where id == 1 returning age
update_stmt
    : UPDATE dotted_name
      SET_KW set_assignment_list
      (WHERE test)?
      returning_clause?
    ;

set_assignment_list
    : set_assignment (COMMA set_assignment)*
    ;

set_assignment
    : dotted_name EQUAL test
    ;

// Postgres-like DELETE: delete from db.users where id == 1 returning count()
delete_stmt
    : DELETE FROM dotted_name
      (WHERE test)?
      returning_clause?
    ;

returning_clause
    : RETURNING select_projection
    ;

// In-Memory Index Specification (e.g. index by (id [hash, unique], email [btree]))
index_clause
    : INDEX BY? LPAR index_field (COMMA index_field)* RPAR
    ;

index_field
    : identifier (LSQB index_type (COMMA index_type)* RSQB)?
    ;

index_type
    : identifier
    ;

index_hint
    : (WITH | AT) INDEX LPAR identifier (COMMA identifier)* RPAR
    ;

// LINQ/Comprehension Query Form:
query_expr
    : from_clause join_clause* where_clause? group_clause? order_clause? select_clause limit_clause?
    ;

from_clause
    : FROM identifier (AS identifier)? IN test index_hint?
    ;

join_clause
    : (INNER | LEFT | RIGHT | FULL | CROSS)? JOIN identifier (AS identifier)? (IN test)? ON test
    ;

where_clause
    : WHERE test
    ;

group_clause
    : GROUP test BY test (INTO identifier)? (HAVING test)?
    ;

order_clause
    : ORDER BY ordering_term (COMMA ordering_term)*
    ;

ordering_term
    : test (ASC | DESC | ASCENDING | DESCENDING)?
    ;

select_clause
    : SELECT DISTINCT? select_projection
    ;

select_projection
    : STAR
    | select_item (COMMA select_item)*
    ;

select_item
    : test (AS identifier)?
    ;

limit_clause
    : LIMIT test (OFFSET test)?
    ;

// SQL-First Query Form:
sql_query_expr
    : SELECT DISTINCT? select_projection FROM from_source join_clause* where_clause? group_clause? order_clause? limit_clause?
    ;

from_source
    : test (AS? identifier)? index_hint?
    ;

// Direct Indexed Key Lookup
lookup_expr
    : LOOKUP test index_hint? WHERE test
    ;

// Set-theoretic operators between datasets / relations
set_op
    : UNION ALL?
    | INTERSECT
    | EXCEPT
    | DIFFERENCE
    ;

// =============================================================================
// Expressions & Precedence Hierarchy
// =============================================================================

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

// Pipelining operators: |> (forward pipe), <| (backward pipe)
pipe_expr
    : set_query_expr ((PIPE_FORWARD | PIPE_BACKWARD) set_query_expr)*
    ;

// Set operations on queries/collections: a union b, a intersect b
set_query_expr
    : coalesce_expr (set_op coalesce_expr)*
    | query_expr (set_op query_expr)*
    | sql_query_expr (set_op sql_query_expr)*
    | lookup_expr
    ;

coalesce_expr
    : or_test (COALESCE or_test | catch_clause)*
    ;

catch_clause
    : CATCH identifier FAT_ARROW (block | test)
    | CATCH block
    | CATCH or_test
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
    | JSON_CONTAINS
    | JSON_CONTAINED
    ;

range_expr
    : bitor_expr ((DOTDOT | DOTDOT_EQUAL | DOTDOT_LESS | ELLIPSIS) bitor_expr)?
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
    : factor ((STAR | SLASH | DOUBLESLASH | PERCENT | JSON_GET_TEXT | ARROW) factor)*
    ;

factor
    : (PLUS | MINUS | TILDE) factor
    | unary_op
    | power
    ;

unary_op
    : AWAIT factor                                                   // Async await
    | ARROW_LEFT factor                                              // Channel receive: <-ch
    | SPAWN (LPAR test RPAR)? factor                                 // Concurrency spawn expression
    | STACKALLOC type_spec (LSQB test RSQB)?                        // Hard real-time stack allocation
    | quote_expr
    | quasiquote_expr
    | splice_expr
    ;

power
    : atom_expr (DOUBLESTAR factor)?
    ;

atom_expr
    : atom trailer*
    ;

trailer
    : LPAR arglist? RPAR over_clause?
    | STAR LPAR arglist? RPAR
    | DOT STAR LPAR arglist? RPAR
    | LSQB subscriptlist RSQB
    | DOT identifier
    | SAFE_NAV identifier
    | SAFE_NAV LSQB subscriptlist RSQB
    | SAFE_NAV LPAR arglist? RPAR
    | QUESTION LSQB subscriptlist RSQB
    | QUESTION LPAR arglist? RPAR
    | QUESTION
    | cascade_trailer
    | LESS unit_expr GREATER
    | over_clause
    ;

cascade_trailer
    : (DOTDOT | SAFE_CASCADE) cascade_target
    ;

cascade_target
    : ( (LET | VAR)? binding_pattern (EQUAL | WALRUS) )? dotted_name (LSQB type_arguments RSQB)? LPAR arglist? RPAR (ARROW binding_pattern)?
    | dotted_name EQUAL test
    | LSQB subscriptlist RSQB EQUAL test
    ;

// Postgres Window Function OVER clause: sum(val) over (partition by dept order by val desc)
over_clause
    : OVER LPAR (PARTITION BY testlist)? (ORDER BY ordering_term (COMMA ordering_term)*)? RPAR
    ;

subscriptlist
    : subscript (COMMA subscript)* COMMA?
    ;

subscript
    : DOTDOT                                                         // ..
    | test DOTDOT test?                                              // a..b or a..
    | DOTDOT test                                                    // ..b
    | test DOTDOT_EQUAL test                                         // a..=b
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
    | if_stmt
    | match_stmt
    | with_stmt
    | for_stmt
    | DOT identifier
    | identifier
    | LPAR testlist_comp? RPAR
    | collection_expr
    | LBRACE dictorsetmaker? RBRACE
    | block
    | sql_query_expr
    | query_expr
    | lookup_expr
    | insert_stmt
    | update_stmt
    | delete_stmt
    | RAW_DSL_BLOCK
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
    | DURATION_LITERAL
    | FREQUENCY_LITERAL
    | MEMORY_LITERAL
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
    : (REF | EACH | REPEAT)? (STAR | DOUBLESTAR | DOTDOT)? test
    | identifier (EQUAL | COLON) (EACH | REPEAT | DOTDOT)? test
    | STAR
    ;


// =============================================================================
// LEXER RULES
// =============================================================================


// Accessibility & Declaration Modifiers
PUBLIC      : 'public';
PRIVATE     : 'private';
PROTECTED   : 'protected';
INTERNAL    : 'internal';
STATIC      : 'static';
PARTIAL     : 'partial';
STEAL       : 'steal';
CONTRACTS   : 'contracts';
DELEGATE    : 'delegate';
PARAMS      : 'params';
ARGS        : 'args';
KWARGS      : 'kwargs';
NAMEOF      : 'nameof';
EACH        : 'each';
REPEAT      : 'repeat';
ADAPTER     : 'adapter';
CONVERTER   : 'converter';

DEF         : 'def';
FN          : 'fn';
CLASS       : 'class';
STRUCT      : 'struct';
RECORD      : 'record';
INTERFACE   : 'interface';
PROTOCOL    : 'protocol';
ENUM        : 'enum';
ACTOR       : 'actor';
LET         : 'let';
VAR         : 'var';
CONST       : 'const';
TYPE        : 'type';
REF         : 'ref';
DYNAMIC     : 'dynamic';
SYMBOL      : 'symbol';
PURE        : 'pure';
MUT         : 'mut';
IMMUT       : 'immut';
PROPERTY    : 'property';
GET         : 'get';
OPERATOR    : 'operator';
EXTENSION   : 'extension';
EXTEND      : 'extend';
STATE       : 'state';

// --- Relational & Postgres-like Database Keywords ---
SELECT      : 'select';
FROM        : 'from';
WHERE       : 'where';
JOIN        : 'join';
INNER       : 'inner';
LEFT        : 'left';
RIGHT       : 'right';
FULL        : 'full';
CROSS       : 'cross';
OUTER       : 'outer';
ON          : 'on';
GROUP       : 'group';
BY          : 'by';
HAVING      : 'having';
ORDER       : 'order';
ASC         : 'asc';
DESC        : 'desc';
ASCENDING   : 'ascending';
DESCENDING  : 'descending';
LIMIT       : 'limit';
OFFSET      : 'offset';
INTO        : 'into';
DISTINCT    : 'distinct';
UNION       : 'union';
ALL         : 'all';
INTERSECT   : 'intersect';
EXCEPT      : 'except';
DIFFERENCE  : 'difference';
INDEX       : 'index';
INDEXED     : 'indexed';
LOOKUP      : 'lookup';
TABLE       : 'table';
SCHEMA      : 'schema';
INSERT      : 'insert';
UPDATE      : 'update';
DELETE      : 'delete';
VALUES      : 'values';
SET_KW      : 'set';
RETURNING   : 'returning';
CONFLICT    : 'conflict';
DO          : 'do';
NOTHING     : 'nothing';
OVER        : 'over';
PARTITION   : 'partition';
WINDOW      : 'window';
TRANSACTION : 'transaction';
COMMIT      : 'commit';
ROLLBACK    : 'rollback';
SAVEPOINT   : 'savepoint';
TO          : 'to';
CONNECT     : 'connect';
DATABASE    : 'database';
MODEL       : 'model';

// --- Asynchronous & Multi-Threaded Concurrency Keywords ---
ASYNC       : 'async';
AWAIT       : 'await';
SPAWN       : 'spawn';
PARALLEL    : 'parallel';
CHAN        : 'chan';
CHANNEL     : 'channel';
ATOMIC      : 'atomic';
SYNC        : 'sync';
LOCK        : 'lock';
ISOLATE     : 'isolate';

// --- Soft Real-Time vs Hard Real-Time Keywords ---
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

// --- Polyglot Interoperability (Python, C#, Rust, JS, Java, ASM, C++, C, Lua, SQL) ---
EXTERN      : 'extern';
CSHARP      : 'csharp';
PYTHON      : 'python';
RUST        : 'rust';
JS          : 'js';
JAVASCRIPT  : 'javascript';
JAVA        : 'java';
ASM         : 'asm';
ASSEMBLY    : 'assembly';
CPP         : 'cpp' | 'c++';
C_KW        : 'c';
LUA         : 'lua';
SQL_KW      : 'sql';
POSTGRES    : 'postgres';
POSTGRESQL  : 'postgresql';
MYSQL       : 'mysql';
MODULE      : 'module';
LIB         : 'lib';

// --- Universal Domain-Specific Abstractions Keywords ---
DSL         : 'dsl';
DOMAIN      : 'domain';
UNIT        : 'unit';
MACRO       : 'macro';
DEFMACRO    : 'defmacro';
QUOTE       : 'quote';
QUASIQUOTE  : 'quasiquote';
SPLICE      : 'splice';
UNQUOTE     : 'unquote';
UNQUOTE_SPLICING : 'unquote_splicing';
MATCH       : 'match';
CASE        : 'case';
WHEN        : 'when';
DEFAULT     : 'default';
TIMEOUT     : 'timeout';

// --- Robust Control Flow & Precondition Keywords ---
GUARD       : 'guard';      // Mandatory early-exit precondition
DEFER       : 'defer';      // Schedule cleanup at scope/function exit
CONTEXT     : 'context';    // Native dependency injection & contextual bindings
REQUIRES    : 'requires';   // Design-by-contract precondition
ENSURES     : 'ensures';    // Design-by-contract postcondition
IF          : 'if';
ELIF        : 'elif';
ELSE        : 'else';
THEN        : 'then';
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
CATCH       : 'catch';
FINALLY     : 'finally';
WITH        : 'with';
AS          : 'as';
ASSERT      : 'assert';
IMPORT      : 'import';
GLOBAL      : 'global';
LAMBDA      : 'lambda';
AND         : 'and' | '&&';
OR          : 'or' | '||';
NOT         : 'not';
IS          : 'is';

// --- Built-in Literals ---
TRUE        : 'True' | 'true';
FALSE       : 'False' | 'false';
SOME        : 'some';
NONE        : 'None' | 'none';
NULL        : 'null';
UNDEFINED   : 'undefined';

// --- Multi-character Operators & Arrows ---
JSON_GET_TEXT    : '->>';    // Postgres JSON extract as text
ARROW_LEFT       : '<-';     // Channel send/receive
ARROW            : '->';     // Return type / mapping / Postgres JSON extract
FAT_ARROW        : '=>';     // Lambda / Match arm
PIPE_FORWARD     : '|>';     // Pipeline forward operator
PIPE_BACKWARD    : '<|';     // Pipeline backward operator
SAFE_CASCADE     : '?..';    // Null-conditional cascade
SAFE_NAV         : '?.';     // Null-conditional navigation
COALESCE_EQUAL   : '??=';    // Null-coalescing assignment
COALESCE         : '??';     // Null coalescing
JSON_CONTAINS    : '@>';     // Postgres JSONB contains
JSON_CONTAINED   : '<@';     // Postgres JSONB contained by
ELLIPSIS         : '...';    // Inclusive / variadic ellipsis
DOTDOT_EQUAL     : '..=';    // Inclusive range
DOTDOT_LESS      : '..<';    // Half-open range
DOTDOT           : '..';     // Range / slice spread
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

// --- Single-character Delimiters & Operators ---
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
DOLLAR      : '$';
BACKQUOTE   : '`';
UNDERSCORE  : '_';

// --- Temporal, Frequency, and Memory Units (Lexical Literals) ---
DURATION_LITERAL
    : [0-9]+ ('.' [0-9]+)? ('ns' | 'us' | 'ms' | 's' | 'min' | 'h')
    ;

FREQUENCY_LITERAL
    : [0-9]+ ('.' [0-9]+)? ('Hz' | 'kHz' | 'MHz' | 'GHz')
    ;

MEMORY_LITERAL
    : [0-9]+ ('B' | 'KB' | 'MB' | 'GB' | 'TB')
    ;

// --- Numeric Literals ---
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

// --- Raw DSL Embedded Blocks (e.g. ```sql ... ```) ---
RAW_DSL_BLOCK
    : '```' [a-zA-Z0-9_]* ~[\r\n]* ('\r'? '\n') .*? '```'
    ;

// --- String Literals ---
TRIPLE_STRING
    : '"""' .*? '"""'
    | '\'\'\'' .*? '\'\'\''
    ;

FSTRING
    : [fF$] ('"""' .*? '"""' | '\'\'\'' .*? '\'\'\'' | '"' (~["\\\r\n] | '\\' .)* '"' | '\'' (~['\\\r\n] | '\\' .)* '\'')
    ;

STRING
    : '"' (~["\\\r\n] | '\\' .)* '"'
    | '\'' (~['\\\r\n] | '\\' .)* '\''
    ;

// --- Identifiers ---
NAME
    : [a-zA-Z_][a-zA-Z0-9_]*
    ;

// --- Whitespace, Line Joining, and Comments ---
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

// Catch-all for unrecognized characters
ERRORTOKEN
    : .
    ;

