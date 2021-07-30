// Test stack layouts.

---
#let rect(width, fill) = rect(width: width, height: 1cm, fill: fill)
#stack(
  rect(2cm, rgb("2a631a")),
  rect(3cm, forest),
  rect(1cm, conifer),
)

---
// Test overflowing stack.

#let rect(width, fill) = rect(width: 1cm, height: 0.4cm, fill: fill)
#box(height: 0.5cm, stack(
  rect(3cm, forest),
  rect(1cm, conifer),
))
