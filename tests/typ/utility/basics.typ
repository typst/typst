// Test basic functions.
// Ref: false

---
// Test the `assert` function.
#assert(1 + 1 == 2)
#assert(2..5 == (2, 3, 4))
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

---
// Test the `repr` function.
#test(repr(ltr), "ltr")
#test(repr((1, 2, false, )), "(1, 2, false)")

---
// Test the `join` function.
#test(join(), none)
#test(join(sep: false), none)
#test(join(1), 1)
#test(join("a", "b", "c"), "abc")
#test("(" + join("a", "b", "c", sep: ", ") + ")", "(a, b, c)")

---
// Test joining templates.
// Ref: true
#join([One], [Two], [Three], sep: [, ]).

---
// Error: 11-24 cannot join boolean with boolean
#test(join(true, false))

---
// Error: 11-29 cannot join string with integer
#test(join("a", "b", sep: 1))

---
// Test conversion functions.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("150"), 150)
#test(type(10 / 3), "float")
#test(int(10 / 3), 3)
#test(float(10), 10.0)
#test(float("31.4e-1"), 3.14)
#test(type(float(10)), "float")
#test(str(123), "123")
#test(str(50.14), "50.14")
#test(len(str(10 / 3)) > 10, true)

---
// Error: 6-10 cannot convert length to integer
#int(10pt)

---
// Error: 8-13 cannot convert function to float
#float(float)

---
// Error: 6-8 cannot convert template to string
#str([])

---
// Error: 6-12 invalid integer
#int("nope")

---
// Error: 8-15 invalid float
#float("1.2.3")
