// Test arrays.

---
// Empty.
{()}

// Not an array, just a parenthesized expression.
{(1)}

// One item and trailing comma.
{(-1,)}

// No trailing comma.
{(true, false)}

// Multiple lines and items and trailing comma.
{("1"
     , #002
     ,)}

// Error: 3 expected closing paren
{(}

// Error: 2-3 expected expression, found closing paren
{)}

// Error: 2:4 expected comma
// Error: 1:4-1:6 expected expression, found end of block comment
{(1*/2)}

// Error: 6-8 expected expression, found invalid token
{(1, 1u 2)}

// Error: 3-4 expected expression, found comma
{(,1)}

// Missing expression makes named pair incomplete, making this an empty array.
// Error: 5 expected expression
{(a:)}

// Named pair after this is already identified as an array.
// Error: 6-10 expected expression, found named pair
{(1, b: 2)}
