// Test whether `relative: "parent"` works correctly
---

#let grad = gradient.linear(red, blue, green, purple, relative: "parent").sharp(4);
#let my_rect = rect(width: 50%, height: 50%, fill: grad, stroke: 1pt)
#set page(
  height: 200pt,
  width: 200pt,
  margin: 0pt,
  fill: gradient.linear(red, blue, green, purple).sharp(4),
  background: [
    #place(top + left, my_rect)
  ]
)
#place(top + right, my_rect)
#place(bottom + center, rotate(45deg, my_rect))
