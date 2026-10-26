grammar DatabaseParser;

import CoreParser, DatabaseLexer, CoreLexer;

// =============================================================================
// DOMAIN: Native & Postgres-like Generic Database Integration (Parser)
// =============================================================================

database_stmt
    : DATABASE identifier type_annotation? EQUAL test
    ;

database_def
    : DATABASE identifier (LPAR testlist? RPAR)? COLON? suite
    ;

model_def
    : decorators? (MODEL | TABLE | SCHEMA) identifier generic_params? (LPAR class_bases? RPAR)? COLON? suite
    ;

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

delete_stmt
    : DELETE FROM dotted_name
      (WHERE test)?
      returning_clause?
    ;

returning_clause
    : RETURNING select_projection
    ;

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

sql_query_expr
    : SELECT DISTINCT? select_projection FROM from_source join_clause* where_clause? group_clause? order_clause? limit_clause?
    ;

from_source
    : test (AS identifier)? index_hint?
    ;

lookup_expr
    : LOOKUP test index_hint? WHERE test
    ;

set_op
    : UNION ALL?
    | INTERSECT
    | EXCEPT
    | DIFFERENCE
    ;

over_clause
    : OVER LPAR (PARTITION BY testlist)? (ORDER BY ordering_term (COMMA ordering_term)*)? RPAR
    ;
