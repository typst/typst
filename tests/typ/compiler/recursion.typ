// Test recursive function calls.
// Ref: false

---
// Test with named function.
#let fib(n) = {
  if n <= 2 {
    1
  } else {
    fib(n - 1) + fib(n - 2)
  }
}

#test(fib(10), 55)

---
// Test with unnamed function.
// Error: 17-18 unknown variable: f
#let f = (n) => f(n - 1)
#f(10)

---
// Test capturing with named function.
#let f = 10
#let f() = f
#test(type(f()), function)

---
// Test capturing with unnamed function.
#let f = 10
#let f = () => f
#test(type(f()), int)

---
// Test redefinition.
#let f(x) = "hello"
#let f(x) = if x != none { f(none) } else { "world" }
#test(f(1), "world")

---
// Error: 15-21 maximum function call depth exceeded
#let rec(n) = rec(n) + 1
#rec(1)

---
// Test cyclic imports during layout.
// Error: 14-37 maximum layout depth exceeded
// Hint: 14-37 try to reduce the amount of nesting in your layout
#layout(_ => include "recursion.typ")

---
// Test recursive show rules.
// Error: 22-25 maximum show rule depth exceeded
// Hint: 22-25 check whether the show rule matches its own output
// Hint: 22-25 this is a current compiler limitation that will be resolved in the future
#show math.equation: $x$
$ x $

---
// Error: 18-21 maximum show rule depth exceeded
// Hint: 18-21 check whether the show rule matches its own output
// Hint: 18-21 this is a current compiler limitation that will be resolved in the future
#show "hey": box[hey]
hey

---
// Error: 14-19 maximum show rule depth exceeded
// Hint: 14-19 check whether the show rule matches its own output
// Hint: 14-19 this is a current compiler limitation that will be resolved in the future
#show "hey": "hey"
hey
