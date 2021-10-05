// Test math functions.
// Ref: false

---
// Test `abs` function.
#test(abs(-3), 3)
#test(abs(3), 3)
#test(abs(-0.0), 0.0)
#test(abs(0.0), -0.0)
#test(abs(-3.14), 3.14)
#test(abs(-12pt), 12pt)
#test(abs(50%), 50%)

---
// Error: 6-16 cannot take absolute value of a linear
#abs(10pt + 50%)

---
// Error: 6-17 expected numeric value, found string
#abs("no number")

---
// Test `min` and `max` functions.
#test(min(2, -4), -4)
#test(min(3.5, 1e2, -0.1, 3), -0.1)
#test(max(-3, 11), 11)
#test(min("hi"), "hi")

---
// Error: 5-7 missing argument: value
#min()

---
// Error: 14-18 cannot compare integer with string
#test(min(1, "hi"), error)
