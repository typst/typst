// Test whether `relative: "parent"` works correctly
// In the first test, the image should look as if there is a single
// gradient that is being used for both the page and the rectangles.
// In the second test, the image should look as if there are multiple
// gradients, one for each rectangle.
---

#let grad = gradient.linear(red, blue, green, purple, relative: "parent");
#let my_rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 200pt,
  width: 200pt,
  fill: grad,
  background: [
    #place(top + left, my_rect)
  ]
)
#place(top + right, my_rect)
#place(bottom + center, rotate(45deg, my_rect))

---

#let grad = gradient.linear(red, blue, green, purple, relative: "self");
#let my_rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 200pt,
  width: 200pt,
  fill: grad,
  background: [
    #place(top + left, my_rect)
  ]
)
#place(top + right, my_rect)
#place(bottom + center, rotate(45deg, my_rect))
