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

#let x = 1
#let f(x, body) = (x, body)
#f!(x)
{ x = 2 }

---
// Test multiple wide calls in one expression.
// Ref: false

#let f() = []
#let g(x, y) = []

// Error: 2-4 wide calls are only allowed directly in templates
{f!()}

// Test nested wide calls.
// Error: 5-7 wide calls are only allowed directly in templates
#g!(f!())

---
// Test missing parentheses.
// Ref: false

// Error: 4 expected argument list
#f!
