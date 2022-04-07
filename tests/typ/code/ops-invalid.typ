// Test invalid operations.
// Ref: false

---
// Error: 3 expected expression
{-}

---
// Error: 10 expected expression
#test({1+}, 1)

---
// Error: 10 expected expression
#test({2*}, 2)

---
// Error: 2-12 cannot apply '+' to content
{+([] + [])}

---
// Error: 2-5 cannot apply '-' to string
{-""}

---
// Error: 2-8 cannot apply 'not' to array
{not ()}

---
// Error: 2-18 cannot apply '<=' to relative length and ratio
{30% + 1pt <= 40%}

---
// Special messages for +, -, * and /.
// Error: 03-10 cannot add integer and string
{(1 + "2", 40% - 1)}

---
// Error: 12-19 cannot subtract integer from ratio
{(1234567, 40% - 1)}

---
// Error: 2-10 cannot multiply integer with boolean
{2 * true}

---
// Error: 2-10 cannot divide integer by length
{3 / 12pt}

---
// Error: 2-9 cannot repeat this string -1 times
{-1 * ""}

---
{
  let x = 2
  for _ in range(61) {
    x *= 2
  }
  // Error: 4-18 cannot repeat this string 4611686018427387904 times
  {x * "abcdefgh"}
}

---
// Error: 14-22 cannot add integer and string
{ let x = 1; x += "2" }

---
// Error: 3-6 cannot mutate a temporary value
{ (x) = "" }

---
// Error: 3-8 cannot mutate a temporary value
{ 1 + 2 += 3 }

---
// Error: 3-4 unknown variable
{ z = 1 }

---
// Error: 3-7 cannot mutate a constant
{ rect = "hi" }

---
// Works if we define rect beforehand
// (since then it doesn't resolve to the standard library version anymore).
#let rect = ""
{ rect = "hi" }
