// Test return out of functions.
// Ref: false

---
#let f(x) = {
  // Error: 3-15 return is not yet implemented
  return x + 1
}

#f(1)
