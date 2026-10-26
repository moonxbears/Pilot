grammar CSharpParser;

import CoreParser, CSharpLexer, CoreLexer;

// ============================================================================= DOMAIN: C#-style
// Robustness & Customization (Parser)
// =============================================================================

property_def:
	decorators? PROPERTY identifier type_annotation? COLON? property_body;

property_body: LBRACE property_accessor* RBRACE;

property_accessor: get_accessor | set_accessor;

get_accessor: GET (COLON? suite | SEMI)?;

set_accessor:
	SET_KW (LPAR identifier RPAR)? (COLON? suite | SEMI)?;

operator_def:
	decorators? OPERATOR overloadable_op parameters (
		ARROW type_spec
	)? COLON? suite;

overloadable_op:
	PLUS
	| MINUS
	| STAR
	| SLASH
	| PERCENT
	| DOUBLESTAR
	| DOUBLESLASH
	| EQEQUAL
	| NOTEQUAL
	| LESS
	| LESSEQUAL
	| GREATER
	| GREATEREQUAL
	| AMPER
	| VBAR
	| CIRCUMFLEX
	| TILDE
	| LEFTSHIFT
	| RIGHTSHIFT
	| LSQB RSQB EQUAL?;

extension_def:
	decorators? EXTENSION identifier? FOR type_spec where_type_clause? COLON? suite;

where_type_clause:
	WHERE where_type_item (COMMA where_type_item)*;

where_type_item: identifier COLON type_spec;
