// Test invalid function calls.

---
// Error: 1-2 unexpected invalid token
#

---
// Error: 4-5 expected expression, found colon
#f(:)

// Error: 7-9 expected expression, found end of block comment
#f(a:1*/)

// Error: 5 expected comma
#f(1 2)

// Error: 2:4-2:5 expected identifier
// Error: 1:6 expected expression
#f(1:)

// Error: 4-5 expected identifier
#f(1:2)

// Error: 4-7 expected identifier
{f((x):1)}

---
#let x = "string"

// Error: 1-3 expected function, found string
#x()

---
// Error: 3:1 expected closing bracket
#f[`a]`

---
// Error: 4 expected closing paren
{f(}

---
// Error: 3:1 expected quote
// Error: 2:1 expected closing paren
#f("]
