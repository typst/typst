// Test math classes.

---
// Test characters.
$ a class(+, "normal") b \
  a class(., "binary") b \
  lr(class(\/, "opening") a/b class(\\, "closing")) \
  { x class(\;, "fence") x > 0} \
  a class(\/, "large") b \
  a class(:, "punctuation") b \
  a class(~, "relation") b \
  a + class(times, "unary") b \
  class(:, "vary") a class(:, "vary") b $

---
// Test custom content.
#let dotsq = square(
  size: 0.7em,
  stroke: 0.5pt,
  align(center+horizon, circle(radius: 0.15em, fill: black))
)

$ a dotsq b \
  a class(dotsq, "vary") b \
  a + class(dotsq, "vary")b \
  a class(dotsq, "punctuation") b $

---
// Test nested.
#let pluseq = $class(class(+, "normal") class(=, "normal"), "binary")$
$ a pluseq 5 $