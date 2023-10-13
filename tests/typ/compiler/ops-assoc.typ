// Test operator associativity.
// Ref: false

---
// Math operators are left-associative.
#test(10 / 2 / 2 == (10 / 2) / 2, true)
#test(10 / 2 / 2 == 10 / (2 / 2), false)
#test(1 / 2 * 3, 1.5)

---
// Assignment is right-associative.
{
  let x = 1
  let y = 2
  x = y = "ok"
  test(x, none)
  test(y, "ok")
}
