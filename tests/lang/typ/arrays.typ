// Empty.
{()}

// Not an array, just a parenthesized expression.
{(1)}

// One item and trailing comma.
{(-1,)}

// No trailing comma.
{(true, false)}

// Multiple lines and items and trailing comma.
{("one"
     , 2
     , #003
     ,)}

// Missing closing paren.
// Error: 3-3 expected closing paren
{(}

// Not an array.
// Error: 2-3 expected expression, found closing paren
{)}

// Missing comma and bad expression.
// Error: 2:4-2:4 expected comma
// Error: 1:4-1:6 expected expression, found end of block comment
{(1*/2)}

// Bad expression.
// Error: 6-8 expected expression, found invalid token
{(1, 1u 2)}

// Leading comma is not an expression.
// Error: 3-4 expected expression, found comma
{(,1)}

// Missing expression makes named pair incomplete, making this an empty array.
// Error: 5-5 expected expression
{(a:)}

// Named pair after this is already identified as an array.
// Error: 6-10 expected expression, found named pair
{(1, b: 2)}
