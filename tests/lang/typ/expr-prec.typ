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

// Assignment binds stronger than boolean operations.
// Error: 2-7 cannot assign to this expression
{not x = "a"}

---
// Parentheses override precedence.
#test((1), 1)
#test((1+2)*-3, -9)

// Error: 15 expected closing paren
#test({(1 + 1}, 2)

---
// Precedence doesn't matter for chained unary operators.
// Error: 2-11 cannot apply '-' to boolean
{-not true}
