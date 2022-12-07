// Test vectors, matrices, and cases.

---
$ v = vec(1, 2+3, 4) $

---
#set vec(delim: "|")
$ vec(1, 2) $

---
// Error: 17-20 expected "(", "[", "{", or "|"
#set vec(delim: "%")

---
$ f(x, y) := cases(
  1 "if" (x dot y)/2 <= 0,
  2 "if" x in NN,
  3 "if" x "is even",
  4 "else",
) $
