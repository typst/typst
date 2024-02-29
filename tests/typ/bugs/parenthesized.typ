// Ref: false
// Test bugs related to destructuring and parenthesized parsing.

---
// https://github.com/typst/typst/issues/1338
#let foo = "foo"
#let bar = "bar"
// Error: 8-9 expected expression, found underscore
// Error: 16-17 expected expression, found underscore
#(foo: _, bar: _)

---
// https://github.com/typst/typst/issues/1342
// Error: 5-8 expected named or keyed pair, found identifier
// Error: 10-13 expected named or keyed pair, found identifier
#(: foo, bar)

---
// https://github.com/typst/typst/issues/1351
// Error: 17-22 expected pattern, found string
#let foo((test: "bar")) = {}

---
// https://github.com/typst/typst/issues/3014
// Error: 8-17 expected expression, found named pair
#(box, fill: red)

---
// https://github.com/typst/typst/issues/3144
#let f(a: 10) = a(1) + 1
#test(f(a: _ => 5), 6)

---
// Error: 17-20 missing argument: pattern parameter
#let f(a: 10) = a() + 1
#f(a: _ => 5)

---
// This wasn't allowed.
#let ((x)) = 1
#test(x, 1)

---
// This also wasn't allowed.
#let ((a, b)) = (1, 2)
#test(a, 1)
#test(b, 2)

---
// This was unintentionally allowed ...
// Error: 9 expected equals sign
#let (a)

---
// ... where this wasn't.
// Error: 12 expected equals sign
#let (a, b)

---
// This wasn't allowed before the bug fix ...
#let f(..) = {}
#f(arg: 1)

---
// ... but this was.
#let f(..x) = {}
#f(arg: 1)

---
// Here, `best` was accessed as a variable, where it shouldn't have.
#{
  (best: _) = (best: "brr")
}

---
// Same here.
#{
  let array = (1, 2, 3, 4)
  (test: array.at(1), best: _) = (test: "baz", best: "brr")
  test(array, (1, "baz", 3, 4))
}

---
// Here, `a` is not duplicate, where it was previously identified as one.
#let f((a: b), (c,), a) = (a, b, c)
#test(f((a: 1), (2,), 3), (3, 1, 2))

---
// Ensure that we can't have non-atomic closures.
#let x = 1
#let c = [#(x) => (1, 2)]
#test(c.children.last(), [(1, 2)]))

---
// Ensure that we can't have non-atomic destructuring.
#let x = 1
#let c = [#() = ()]
#test(c.children.last(), [()])
