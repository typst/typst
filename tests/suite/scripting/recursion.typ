// Test recursive function calls.

--- recursion-named ---
// Test with named function.
#let fib(n) = {
  if n <= 2 {
    1
  } else {
    fib(n - 1) + fib(n - 2)
  }
}

#test(fib(10), 55)

--- recursion-unnamed-invalid ---
// Test with unnamed function.
// Error: 17-18 unknown variable: f
#let f = (n) => f(n - 1)
#f(10)

--- recursion-named-returns-itself ---
// Test capturing with named function.
#let f = 10
#let f() = f
#test(type(f()), function)

--- recursion-unnamed-does-not-return-itself ---
// Test capturing with unnamed function.
#let f = 10
#let f = () => f
#test(type(f()), int)

--- recursion-shadowing ---
// Test redefinition.
#let f(x) = "hello"
#let f(x) = if x != none { f(none) } else { "world" }
#test(f(1), "world")

--- recursion-maximum-depth ---
// Error: 15-21 maximum function call depth exceeded
#let rec(n) = rec(n) + 1
#rec(1)

--- recursion-via-include-in-layout ---
// Test cyclic imports during layout.
// Error: 2-38 maximum show rule depth exceeded
// Hint: 2-38 check whether the show rule matches its own output
#layout(_ => include "recursion.typ")

--- recursion-show-math ---
// Test recursive show rules.
// Error: 22-25 maximum show rule depth exceeded
// Hint: 22-25 check whether the show rule matches its own output
#show math.equation: $x$
$ x $

--- recursion-show-math-realize ---
// Error: 22-33 maximum show rule depth exceeded
// Hint: 22-33 check whether the show rule matches its own output
#show heading: it => heading[it]
$ #heading[hi] $
