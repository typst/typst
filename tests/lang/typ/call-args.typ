// Test function call arguments.

---
// One argument.
#[f bold]

// One argument and trailing comma.
#[f 1,]

// One named argument.
#[f a:2]

// Mixed arguments.
{f(1, a: (3, 4), 2, b: "5")}

---
// Error: 5-6 expected expression, found colon
#[f :]

// Error: 8-10 expected expression, found end of block comment
#[f a:1*/]

// Error: 6-6 expected comma
#[f 1 2]

// Error: 2:5-2:6 expected identifier
// Error: 1:7-1:7 expected expression
#[f 1:]

// Error: 5-6 expected identifier
#[f 1:2]

// Error: 4-7 expected identifier
{f((x):1)}
