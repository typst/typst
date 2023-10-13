// Test vectors.

---
// Test wide cell.
$ v = vec(1, 2+3, 4) $

---
// Test alternative delimiter.
#set math.vec(delim: "[")
$ vec(1, 2) $

---
// Error: 22-25 expected "(", "[", "{", "|", "||", or none
#set math.vec(delim: "%")
