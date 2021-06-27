// Test stack layouts.

---
#let rect(width, color) = rect(width: width, height: 1cm, fill: color)
#stack(
    rect(2cm, #2a631a),
    rect(3cm, forest),
    rect(1cm, conifer),
)

---

#let rect(width, color) = rect(width: 1cm, height: 0.4cm, fill: color)
// This stack overflows.
#box(height: 0.5cm, stack(
    rect(3cm, forest),
    rect(1cm, conifer),
))
