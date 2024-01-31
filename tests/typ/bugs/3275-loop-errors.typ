// Issue #3275: clearer errors for loops, https://github.com/typst/typst/issues/3275
// Ref: false

---
// Normal variable.
#for x in (1, 2) {}
#for x in (a: 1, b: 2) {}
#for x in "foo" {}

---
// Placeholder.
#for _ in (1, 2) {}
#for _ in (a: 1, b: 2) {}
#for _ in "foo" {}

---
// Destructuring.
#for (k, v)  in (("a", 1), ("b", 2), ("c", 3)) {}
#for (k, ..) in (("a", 1), ("b", 2), ("c", 3)) {}
#for (k, v)  in (a: 1, b: 2, c: 3) {}
#for (.., v) in (a: 1, b: 2, c: 3) {}

---
// Error: 11-17 cannot loop over content
#for x in [1, 2] {}

---
// Error: 11-25 cannot loop over bytes
#for _ in bytes((22, 0)) {}

---
// Error: 16-21 cannot loop over integer
#for (x, y) in 12306 {}

---
// Error: 16-22 cannot loop over content
#for (x, y) in [1, 2] {}

---
// Error: 6-12 cannot destructure values of string
#for (x, y) in "foo" {}

---
// Error: 6-12 cannot destructure string
#for (x, y) in ("foo", "bar") {}

---
// Error: 6-12 cannot destructure integer
#for (x, y) in (1, 2) {}
