// Test basic functions.
// Ref: false

---
// Test the `assert` function.
#assert(1 + 1 == 2)
#assert(range(2, 5) == (2, 3, 4))
#assert(not false)

---
// Test failing assertions.
// Error: 9-15 assertion failed
#assert(1 == 2)

---
// Test failing assertions.
// Error: 9-15 expected boolean, found string
#assert("true")

---
// Test the `type` function.
#test(type(1), "integer")
#test(type(ltr), "direction")
#test(type(10 / 3), "float")
