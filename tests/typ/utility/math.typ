// Test math functions.
// Ref: false

---
// Test conversion to numbers.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("150"), 150)
#test(int(10 / 3), 3)
#test(float(10), 10.0)
#test(float("31.4e-1"), 3.14)
#test(type(float(10)), "float")

---
// Error: 6-10 cannot convert length to integer
#int(10pt)

---
// Error: 8-13 cannot convert function to float
#float(float)

---
// Error: 6-12 invalid integer
#int("nope")

---
// Error: 8-15 invalid float
#float("1.2.3")

---
// Test the `abs` function.
#test(abs(-3), 3)
#test(abs(3), 3)
#test(abs(-0.0), 0.0)
#test(abs(0.0), -0.0)
#test(abs(-3.14), 3.14)
#test(abs(-12pt), 12pt)
#test(abs(50%), 50%)

---
// Test the `even` and `odd` functions.
#test(even(2), true)
#test(odd(2), false)
#test(odd(-1), true)
#test(even(-11), false)

---
// Test the `mod` function.
#test(mod(1, 1), 0)
#test(mod(5, 3), 2)
#test(mod(5, -3), 2)
#test(mod(22.5, 10), 2.5)
#test(mod(9, 4.5), 0)

---
// Error: 9-10 divisor must not be zero
#mod(5, 0)

---
// Error: 11-14 divisor must not be zero
#mod(3.0, 0.0)

---
// Error: 6-16 cannot take absolute value of a linear
#abs(10pt + 50%)

---
// Error: 6-17 expected numeric value, found string
#abs("no number")

---
// Test the `min` and `max` functions.
#test(min(2, -4), -4)
#test(min(3.5, 1e2, -0.1, 3), -0.1)
#test(max(-3, 11), 11)
#test(min("hi"), "hi")

---
// Error: 5-7 missing argument: value
#min()

---
// Error: 9-13 cannot compare integer and string
#min(1, "hi")

---
// Test the `range` function.
#test(range(4), (0, 1, 2, 3))
#test(range(1, 4), (1, 2, 3))
#test(range(-4, 2), (-4, -3, -2, -1, 0, 1))
#test(range(10, 5), ())
#test(range(10, step: 3), (0, 3, 6, 9))
#test(range(1, 4, step: 1), (1, 2, 3))
#test(range(1, 8, step: 2), (1, 3, 5, 7))
#test(range(5, 2, step: -1), (5, 4, 3))
#test(range(10, 0, step: -3), (10, 7, 4, 1))

---
// Error: 7-9 missing argument: end
#range()

---
// Error: 11-14 expected integer, found float
#range(1, 2.0)

---
// Error: 17-22 expected integer, found string
#range(4, step: "one")

---
// Error: 18-19 step must not be zero
#range(10, step: 0)
