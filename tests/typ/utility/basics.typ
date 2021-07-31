// Test basic functions.
// Ref: false

---
// Test the `len` function.
#test(len(()), 0)
#test(len(("A", "B", "C")), 3)
#test(len("Hello World!"), 12)
#test(len((a: 1, b: 2)), 2)

---
// Error: 5-7 missing argument: collection
#len()

---
// Error: 6-10 expected string, array or dictionary
#len(12pt)
