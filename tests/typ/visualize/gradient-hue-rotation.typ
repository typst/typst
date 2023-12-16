// Tests whether hue rotation works correctly.

---
// Test in Oklab space for reference.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: oklab)
)

---
// Test in OkLCH space.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: oklch)
)

---
// Test in HSV space.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: color.hsv)
)

---
// Test in HSL space.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: color.hsl)
)


---
// Test in Oklab space for reference.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: oklab)
)

---
// Test in OkLCH space.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: oklch)
)

---
// Test in HSV space.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: color.hsv)
)

---
// Test in HSL space.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: color.hsl)
)
