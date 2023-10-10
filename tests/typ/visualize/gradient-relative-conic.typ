// Test whether `relative: "parent"` works correctly on conic gradients.

---
// The image should look as if there is a single gradient that is being used for
// both the page and the rectangles.
#let grad = gradient.conic(red, blue, green, purple, relative: "parent");
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 200pt,
  width: 200pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

---
// The image should look as if there are multiple gradients, one for each
// rectangle.
#let grad = gradient.conic(red, blue, green, purple, relative: "self");
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 200pt,
  width: 200pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))
