// Test operator precedence.
// Ref: false

---
// Multiplication binds stronger than addition.
#test(1+2*-3, -5)

// Subtraction binds stronger than comparison.
#test(3 == 5 - 2, true)

// Boolean operations bind stronger than '=='.
#test("a" == "a" and 2 < 3, true)
#test(not "b" == "b", false)

---
// Assignment binds stronger than boolean operations.
// Error: 2:3-2:8 cannot mutate a temporary value
#let x = false
#(not x = "a")

---
// Precedence doesn't matter for chained unary operators.
// Error: 3-12 cannot apply '-' to boolean
#(-not true)

---
// Not in handles precedence.
#test(-1 not in (1, 2, 3), true)

---
// Parentheses override precedence.
#test((1), 1)
#test((1+2)*-3, -9)

// Error: 8-9 unclosed delimiter
#test({(1 + 1}, 2)
