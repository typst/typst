// Test vectors, matrices, and cases.

---
$ v = vec(1, 2+3, 4) $

---
$ binom(n, 1) = 1/2 n (n-1) $

---
#set math.vec(delim: "|")
$ vec(1, 2) $

---
$ f(x, y) := cases(
  1 "if" (x dot y)/2 <= 0,
  2 "if" x in NN,
  3 "if" x "is even",
  4 "else",
) $

---
// Error: 22-25 expected "(", "[", "{", "|", or "||"
#set math.vec(delim: "%")

---
// Error: 9-12 missing argument: lower index
$ binom(x^2) $
