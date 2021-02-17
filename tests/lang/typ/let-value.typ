// Test value of let binding.
// Ref: false

---
// Automatically initialized with none.
#let x
#test(x, none)

// Manually initialized with one.
#let x = 1
#test(x, 1)
