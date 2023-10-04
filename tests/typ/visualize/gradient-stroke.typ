// Test gradients on strokes.

---
#set page(width: 100pt, height: auto, margin: 0pt)
#align(center + top, square(size: 50pt, fill: black, stroke: 5pt + gradient.linear(red, blue)))
#align(center + bottom, square(size: 50pt, fill: black, stroke: 5pt + gradient.radial(red, blue, radius: 70.7%, start-radius: 50%)))

---
// Test gradient on lines
#set page(width: 100pt, height: 100pt)
#line(length: 100%, stroke: 1pt + gradient.linear(red, blue))
#line(length: 100%, angle: 10deg, stroke: 1pt + gradient.linear(red, blue))
#line(length: 100%, angle: 10deg, stroke: 1pt + gradient.linear(red, blue, relative: "parent"))
