// Test invalid function calls.

---
// Error: 1-2 unexpected invalid token
#

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
