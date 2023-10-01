// Test whether `relative: "parent"` works correctly
// In the first test, the image should look as if there is a single
// gradient that is being used for both the page and the rectangles.
// In the second test, the image should look as if there are multiple
// gradients, one for each rectangle.
---

#let grad = gradient.linear(red, blue, green, purple, relative: "parent");
#let my_rect = rect(width: 50pt, height: 50pt, fill: grad)
#set page(
  height: 200pt,
  width: 200pt,
)
#place(top + right, scale(x: 200%, y: 130%, my_rect))
#place(bottom + center, rotate(45deg, my_rect))
#place(horizon + center, scale(x: 200%, y: 130%, rotate(45deg, my_rect)))