// Test embedded expressions.
// Ref: false

---
// Error: 6-8 expected identifier, found keyword `as`
#let as = 1 + 2

---
#{
  // Error: 7-9 expected identifier, found keyword `as`
  let as = 10
}
