lexer grammar DatabaseLexer;

// =============================================================================
// DOMAIN: Native & Postgres-like Generic Database Integration (Lexer)
// =============================================================================

DATABASE    : 'database';
CONNECT     : 'connect';
MODEL       : 'model';
TABLE       : 'table';
SCHEMA      : 'schema';
INDEX       : 'index';
INDEXED     : 'indexed';
LOOKUP      : 'lookup';

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
DISTINCT    : 'distinct';

INSERT      : 'insert';
INTO        : 'into';
VALUES      : 'values';
UPDATE      : 'update';
SET_KW      : 'set';
DELETE      : 'delete';
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

UNION       : 'union';
ALL         : 'all';
INTERSECT   : 'intersect';
EXCEPT      : 'except';
DIFFERENCE  : 'difference';

JSON_GET_TEXT   : '->>';
JSON_CONTAINS   : '@>';
JSON_CONTAINED  : '<@';
