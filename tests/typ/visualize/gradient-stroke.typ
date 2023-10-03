// Test gradients on strokes.

---
#set page(width: 100pt, height: 100pt)
#align(center + horizon, square(size: 50pt, fill: black, stroke: 5pt + gradient.linear(red, blue)))

---
// Test gradient on lines
#set page(width: 100pt, height: 100pt)
#line(length: 100%, stroke: 1pt + gradient.linear(red, blue))
#line(length: 100%, angle: 10deg, stroke: 1pt + gradient.linear(red, blue))
#line(length: 100%, angle: 10deg, stroke: 1pt + gradient.linear(red, blue, relative: "parent"))
