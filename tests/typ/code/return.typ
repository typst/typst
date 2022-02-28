// Test return out of functions.
// Ref: false

---
#let f(x) = {
  return x + 1
}

#test(f(1), 2)

---
// Test return outside of function.

#for x in range(5) {
  // Error: 3-9 cannot return outside of function
  return
}
