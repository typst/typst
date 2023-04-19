// Test case distinction.

---
$ f(x, y) := cases(
  1 quad &"if" (x dot y)/2 <= 0,
  2 &"if" x divides 2,
  3 &"if" x in NN,
  4 &"else",
) $
