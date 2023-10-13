// Test math classes.

---
// Test characters.
$ a class("normal", +) b \
  a class("binary", .) b \
  lr(class("opening", \/) a/b class("closing", \\)) \
  { x class("fence", \;) x > 0} \
  a class("large", \/) b \
  a class("punctuation", :) b \
  a class("relation", ~) b \
  a + class("unary", times) b \
  class("vary", :) a class("vary", :) b $

---
// Test custom content.
#let dotsq = square(
  size: 0.7em,
  stroke: 0.5pt,
  align(center+horizon, circle(radius: 0.15em, fill: black))
)

$ a dotsq b \
  a class("normal", dotsq) b \
  a class("vary", dotsq) b \
  a + class("vary", dotsq) b \
  a class("punctuation", dotsq) b $

---
// Test nested.
#let normal = math.class.with("normal")
#let pluseq = $class("binary", normal(+) normal(=))$
$ a pluseq 5 $

---
// Test exceptions.
$ sqrt(3)\/2 quad d_0.d_1d_2 dots $