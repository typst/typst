// Empty
{(:)}

// Two pairs.
{(one: 1, two: 2)}

---
// Test errors.
//
// error: 2:9-2:10 expected named pair, found expression
// error: 4:4-4:5 expected named pair, found expression
// error: 4:5-4:5 expected comma
// error: 4:12-4:16 expected identifier
// error: 4:17-4:18 expected expression, found colon

{(a: 1, b)}

{(:1 b:[], true::)}
