// Test recursive function calls.

--- recursion-named render ---
// Test with named function.
#let fib(n) = {
  if n <= 2 {
    1
  } else {
    fib(n - 1) + fib(n - 2)
  }
}

#test(fib(10), 55)

--- recursion-unnamed-invalid render ---
// Test with unnamed function.
// Error: 17-18 unknown variable: f
#let f = (n) => f(n - 1)
#f(10)

--- recursion-named-returns-itself render ---
// Test capturing with named function.
#let f = 10
#let f() = f
#test(type(f()), function)

--- recursion-unnamed-does-not-return-itself render ---
// Test capturing with unnamed function.
#let f = 10
#let f = () => f
#test(type(f()), int)

--- recursion-shadowing render ---
// Test redefinition.
#let f(x) = "hello"
#let f(x) = if x != none { f(none) } else { "world" }
#test(f(1), "world")

--- recursion-maximum-depth render ---
// Error: 15-21 maximum function call depth exceeded
#let rec(n) = rec(n) + 1
#rec(1)

--- recursion-via-include-in-layout render ---
// Test cyclic imports during layout.
// Error: 2-38 maximum show rule depth exceeded
// Hint: 2-38 maybe a show rule matches its own output
// Hint: 2-38 maybe there are too deeply nested elements
#layout(_ => include "recursion.typ")

--- recursion-show-math render ---
// Test recursive show rules.
// Error: 22-25 maximum show rule depth exceeded
// Hint: 22-25 maybe a show rule matches its own output
// Hint: 22-25 maybe there are too deeply nested elements
#show math.equation: $x$
$ x $

--- recursion-show-math-realize render ---
// Error: 22-33 maximum show rule depth exceeded
// Hint: 22-33 maybe a show rule matches its own output
// Hint: 22-33 maybe there are too deeply nested elements
#show heading: it => heading[it]
$ #heading[hi] $
