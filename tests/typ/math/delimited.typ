// Test delimiter matching and scaling.

---
// Test automatic matching.
$ (a) + {b/2} + |a|/2 + (b) $
$f(x/2) < zeta(c^2 + |a + b/2|)$

---
// Test unmatched.
$[1,2[ = [1,2) != zeta\(x/2\) $

---
// Test manual matching.
$ [|a/b|] != lr(|]a/b|]) != [a/b) $
$ lr(| ]1,2\[ + 1/2|) $

---
// Test fence confusion.
$ |x + |y| + z/a| \
  |x + lr(|y|) + z/a| $

---
// Test that symbols aren't matched automatically.
$ bracket.l a/b bracket.r
  = lr(bracket.l a/b bracket.r) $

---
// Test half LRs.
$ lr(a/b\]) = a = lr(\{a/b) $

---
// Test manual scaling.
$ lr(]sum_(x=1)^n x], size: #70%)
  < lr((1, 2), size: #200%) $

---
// Test predefined delimiter pairings.
$floor(x/2), ceil(x/2), abs(x), norm(x)$

---
// Test colored delimiters
$ lr(
    text("(", fill: #green) a/b
    text(")", fill: #blue)
  ) $
