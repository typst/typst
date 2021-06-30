// Test wide calls.

---
// Test multiple wide calls in separate expressions.
#font!(color: eastern) - First
#font!(color: forest) - Second

---
// Test in heading.
# A #align!(right) B
C

---
// Test evaluation semantics.
// Ref: false

#let r
#let x = 1
#let f(x, body) = (x, body)

[
  { r = f!(x) }
  { x = 2 }
]

#test(repr(r), "(1, <template>)")

---
// Test multiple wide calls in one expression.
// Ref: false

#let id(x) = x
#let add(x, y) = x + y

// Error: 11-13 duplicate wide call
[{id!() + id!()}]

// Test nested wide calls.
// Error: 2-6 duplicate wide call
[#add!(id!())]

---
// Test missing parentheses.
// Ref: false

// Error: 4 expected argument list
#f!
