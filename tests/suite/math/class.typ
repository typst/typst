// Test math classes.

--- math-class-chars ---
// Test characters.
$ a class("normal", +) b \
  a class("binary", .) b \
  lr(class("opening", \/) a/b class("closing", \\)) \
  { x class("fence", \;) x > 0} \
  a class("large", \/) b \
  a class("punctuation", :) b \
  a class("relation", !) b \
  a + class("unary", times) b \
  class("vary", :) a class("vary", :) b $

--- math-class-content ---
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

--- math-class-nested ---
// Test nested.
#let normal = math.class.with("normal")
#let pluseq = $class("binary", normal(+) normal(=))$
$ a pluseq 5 $

--- math-class-exceptions ---
// Test exceptions.
$ sqrt(3)\/2 quad d_0.d_1d_2 dots $

--- math-class-limits ---
// Test if the math class changes the limit configuration.
$ class("normal", ->)_a $
$class("relation", x)_a$
$ class("large", x)_a $
$class("large", ->)_a$

$limits(class("normal", ->))_a$
$ scripts(class("relation", x))_a $

--- issue-4985-up-tack-is-normal-perp-is-relation ---
$ top = 1 \
  bot = 2 \
  a perp b $
