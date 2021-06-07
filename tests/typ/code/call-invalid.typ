// Test invalid function calls.

---
// Error: 7-8 expected expression, found colon
#args(:)

// Error: 10-12 expected expression, found end of block comment
#args(a:1*/)

// Error: 8 expected comma
#args(1 2)

// Error: 2:7-2:8 expected identifier
// Error: 1:9 expected expression
#args(1:)

// Error: 7-8 expected identifier
#args(1:2)

// Error: 7-10 expected identifier
{args((x):1)}

---
#let x = "string"

// Error: 1-3 expected function, found string
#x()

---
// Error: 3:1 expected closing bracket
#args[`a]`

---
// Error: 7 expected closing paren
{args(}

---
// Error: 3:1 expected quote
// Error: 2:1 expected closing paren
#args("]
