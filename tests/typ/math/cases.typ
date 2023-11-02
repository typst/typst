// Test case distinction.

---
$ cases(
  x, "if" (x dot y)/2 <= 0,
  x + y, "if" x divides 2,
  x / y, "if" x in NN,
  0, "else",
) $

---
#set math.cases(separator: $,quad$)
$ cases(
  x, "if" (x dot y)/2 <= 0,
  x + y, "if" x divides 2,
  x / y, "if" x in NN,
  0, "else",
) $

---
// Error: 8-17 missing case condition
$ cases(x, y, z) $
