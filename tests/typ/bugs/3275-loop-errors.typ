// Issue #3275: clearer errors for loops, https://github.com/typst/typst/issues/3275
// Ref: false

---
#for x in (1, 2, 3) {}
#for x in (a:1, b:2, c:3) {}
#for x in "foo" {}
#for _ in (1, 2, 3) {}
#for _ in (a: 1, b: 2, c: 3) {}
#for _ in "foo" {}
#for (x, y) in (("a", 1), ("b", 2), ("c", 3)) {}
#for (x, y) in (a: 1, b: 2, c: 3) {}
---
// Error: 11-16 cannot loop over integer
#for x in 12306 {}
---
// Error: 11-16 cannot loop over integer
#for _ in 12306 {}
---
// Error: 16-21 cannot loop over integer
#for (x, y) in 12306 {}
---
// Error: 6-12 cannot destructure values of string
#for (x, y) in "foo" {}
---
// Error: 6-12 cannot destructure string
#for (x, y) in ("foo", "bar", "baz") {}
