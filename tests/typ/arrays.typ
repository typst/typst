// Empty.
{()}

// One item and trailing comma.
{(-1,)}

// No trailing comma.
{(true, false)}

// Multiple lines and items and trailing comma.
{("one"
     , 2
     , #003
     ,)}

// Error: 1:3-1:3 expected closing paren
{(}

// Error: 2:4-2:6 expected expression, found end of block comment
// Error: 1:4-1:4 expected comma
{(1*/2)}

// Error: 1:6-1:8 expected expression, found invalid token
{(1, 1u 2)}

// Error: 1:3-1:4 expected expression, found comma
{(,1)}

// Error: 1:5-1:5 expected expression
{(a:)}

// Error: 1:6-1:10 expected expression, found named pair
{(1, b: 2)}
