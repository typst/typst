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
#let plusdot = square(
  size: 0.7em,
  stroke: 0.5pt,
  align(center+horizon, circle(radius: 0.15em, fill: black))
)

$ a plusdot b \
  a class(plusdot, "vary") b \
  a + class(plusdot, "vary")b \
  a class(plusdot, "punctuation") b $