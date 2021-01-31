// Test invalid function calls.

---
// Error: 4-4 expected closing paren
{f(}

---
// Error: 4:1-4:1 expected identifier
// Error: 3:1-3:1 expected closing bracket
#[

---
// Error: 3-3 expected identifier
#[]

// Error: 3-6 expected identifier, found string
#["f"]

// Error: 2:3-2:4 expected identifier, found opening paren
// Error: 1:5-1:6 expected expression, found closing paren
#[(f)]

---
#let x = "string"

// Error: 3-4 expected function, found string
#[x]

---
// Error: 3:1-3:1 expected closing bracket
#[f][`a]`

---
// Error: 3:1-3:1 expected quote
// Error: 2:1-2:1 expected closing bracket
#[f "]
