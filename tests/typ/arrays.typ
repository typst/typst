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

---
// Test errors.
//
// error: 2:3-2:3 expected closing paren
// error: 4:4-4:6 expected expression, found end of block comment
// error: 4:4-4:4 expected comma
// error: 6:6-6:8 expected expression, found invalid token
// error: 8:3-8:4 expected expression, found comma
// error: 10:5-10:5 expected expression
// error: 12:6-12:10 expected expression, found named pair

{(}

{(1*/2)}

{(1, 1u 2)}

{(,1)}

{(a:)}

{(1, b: 2)}
