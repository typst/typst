// Test invalid let binding syntax.

---
// Error: 5-5 expected identifier
#let

// Error: 6-9 expected identifier, found string
#let "v"

// Should output `1`.
// Error: 7-7 expected semicolon or line break
#let v 1

// Error: 9-9 expected expression
#let v =

---
// Should output `= 1`.
// Error: 6-9 expected identifier, found string
#let "v" = 1
