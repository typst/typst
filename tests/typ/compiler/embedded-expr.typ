// Test embedded expressions.
// Ref: false

---
// Error: 6-8 expected identifier, found keyword `as`
// Hint: 6-8 keyword `as` is not allowed as an identifier, try `_as`
#let as = 1 + 2

---
#{
  // Error: 7-9 expected identifier, found keyword `as`
  // Hint: 7-9 keyword `as` is not allowed as an identifier, try `_as`
  let as = 10
}

---
// Error: 2-2 expected expression
#

---
// Error: 2-2 expected expression
#  hello
