// Test row groups.

---
$ f(x, y) := group(
  1 quad &"if" (x dot y)/2 <= 0,
  2 &"if" x divides 2,
  3 &"if" x in NN,
  4 &"else",
) $

---
$
  group(
    -x &+ 3y &+ 8&z &= 4,
    5x &- 2y &+ 2&z &= 3,
    x &+ y &+ &z &= 11,
  )
$
