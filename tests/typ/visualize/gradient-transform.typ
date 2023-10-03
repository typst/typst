// Test whether gradients work well when they are contained within a transform.

---
#let grad = gradient.linear(red, blue, green, purple, relative: "parent");
#let my-rect = rect(width: 50pt, height: 50pt, fill: grad)
#set page(
  height: 200pt,
  width: 200pt,
)
#place(top + right, scale(x: 200%, y: 130%, my-rect))
#place(bottom + center, rotate(45deg, my-rect))
#place(horizon + center, scale(x: 200%, y: 130%, rotate(45deg, my-rect)))
