// Test invalid expressions.
// Ref: false

---
// Missing expressions.

// Error: 3 expected expression
{-}

// Error: 10 expected expression
#test({1+}, 1)

// Error: 10 expected expression
#test({2*}, 2)

---
// Mismatched types.

// Error: 2-12 cannot apply '+' to template
{+([] + [])}

// Error: 2-5 cannot apply '-' to string
{-""}

// Error: 2-8 cannot apply 'not' to array
{not ()}

// Error: 1:2-1:12 cannot apply '<=' to relative and relative
{30% <= 40%}

// Special messages for +, -, * and /.
// Error: 4:03-4:10 cannot add integer and string
// Error: 3:12-3:19 cannot subtract integer from relative
// Error: 2:21-2:29 cannot multiply integer with boolean
// Error: 1:31-1:39 cannot divide integer by length
{(1 + "2", 40% - 1, 2 * true, 3 / 12pt)}

// Error: 14-22 cannot apply '+=' to integer and string
{ let x = 1; x += "2" }

---
// Bad left-hand sides of assignment.

// Error: 3-6 cannot assign to this expression
{ (x) = "" }

// Error: 3-8 cannot assign to this expression
{ 1 + 2 += 3 }

// Error: 3-4 unknown variable
{ z = 1 }

// Error: 3-6 cannot assign to a constant
{ box = "hi" }

// Works if we define box beforehand
// (since then it doesn't resolve to the standard library version anymore).
#let box = ""
{ box = "hi" }
