// Test basic calculation functions.
// Ref: false

---
// Test `min` and `max` functions.
#test(min(2, -4), -4)
#test(min(3.5, 1e2, -0.1, 3), -0.1)
#test(max(-3, 11), 11)
#test(min("hi"), "hi")

// Error: 6 missing argument: value
#min()

// Error: 11-18 cannot compare integer with string
#test(min(1, "hi"), error)
