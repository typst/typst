// Issue #3275: clearer errors for loops, https://github.com/typst/typst/issues/3275
// Ref: false

---
// Normal variable.
#for x in (1, 2) {}
#for x in (a: 1, b: 2) {}
#for x in "foo" {}
#for x in bytes("😊") {}

---
// Placeholder.
#for _ in (1, 2) {}
#for _ in (a: 1, b: 2) {}
#for _ in "foo" {}
#for _ in bytes("😊") {}

---
// Destructuring.
#for (a,b,c) in (("a", 1, bytes(())), ("b", 2, bytes(""))) {}
#for (a, ..) in (("a", 1, bytes(())), ("b", 2, bytes(""))) {}
#for (k, v)  in (a: 1, b: 2, c: 3) {}
#for (.., v) in (a: 1, b: 2, c: 3) {}

---
// Error: 11-17 cannot loop over content
#for x in [1, 2] {}

---
// Error: 11-25 cannot loop over arguments
#for _ in arguments("a") {}

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
// Error: 6-12 cannot destructure values of bytes
#for (x, y) in bytes("😊") {}

---
// Error: 6-12 cannot destructure bytes
#for (x, y) in (bytes((1,2)), bytes((1,2))) {}

---
// Error: 6-12 cannot destructure integer
#for (x, y) in (1, 2) {}

---
// Error: 10-11 not enough elements to destructure
#for (x, y) in ((1,), (2,)) {}

---
// Error: 6-12 too many elements to destructure
#for (x, y) in ((1,2,3), (4,5,6)) {}
