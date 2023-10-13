// Test invalid operations.
// Ref: false

---
// Error: 4 expected expression
#(-)

---
// Error: 10 expected expression
#test({1+}, 1)

---
// Error: 10 expected expression
#test({2*}, 2)

---
// Error: 3-13 cannot apply '+' to content
#(+([] + []))

---
// Error: 3-6 cannot apply '-' to string
#(-"")

---
// Error: 3-9 cannot apply 'not' to array
#(not ())

---
// Error: 3-19 cannot compare relative length and ratio
#(30% + 1pt <= 40%)

---
// Error: 3-14 cannot compare 1em with 10pt
#(1em <= 10pt)

---
// Error: 3-22 cannot compare 2.2 with NaN
#(2.2 <= float("nan"))

---
// Error: 3-12 cannot divide by zero
#(1.2 / 0.0)

---
// Error: 3-8 cannot divide by zero
#(1 / 0)

---
// Error: 3-15 cannot divide by zero
#(15deg / 0deg)

---
// Special messages for +, -, * and /.
// Error: 3-10 cannot add integer and string
#(1 + "2", 40% - 1)

---
// Error: 15-23 cannot add integer and string
#{ let x = 1; x += "2" }

---
// Error: 4-13 cannot divide ratio by length
#( 10% / 5pt )

---
// Error: 3-12 cannot divide these two lengths
#(1em / 5pt)

---
// Error: 3-19 cannot divide relative length by ratio
#((10% + 1pt) / 5%)

---
// Error: 3-28 cannot divide these two relative lengths
#((10% + 1pt) / (20% + 1pt))

---
// Error: 13-20 cannot subtract integer from ratio
#((1234567, 40% - 1))

---
// Error: 3-11 cannot multiply integer with boolean
#(2 * true)

---
// Error: 3-11 cannot divide integer by length
#(3 / 12pt)

---
// Error: 3-10 number must be at least zero
#(-1 * "")

---
// Error: 4-5 unknown variable: x
#((x) = "")

---
// Error: 4-5 unknown variable: x
#((x,) = (1,))

---
// Error: 3-8 cannot mutate a temporary value
#(1 + 2 += 3)

---
// Error: 2:3-2:8 cannot apply 'not' to string
#let x = "Hey"
#(not x = "a")

---
// Error: 7-8 unknown variable: x
#(1 + x += 3)

---
// Error: 3-4 unknown variable: z
#(z = 1)

---
// Error: 3-7 cannot mutate a constant: rect
#(rect = "hi")

---
// Works if we define rect beforehand
// (since then it doesn't resolve to the standard library version anymore).
#let rect = ""
#(rect = "hi")
