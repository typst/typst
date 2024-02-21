// Tests small patterns for pixel accuracy.

---
#box(
  width: 8pt,
  height: 1pt,
  fill: pattern(size: (1pt, 1pt), square(size: 1pt, fill: black))
)
#v(-1em)
#box(
  width: 8pt,
  height: 1pt,
  fill: pattern(size: (2pt, 1pt), square(size: 1pt, fill: black))
)

---
// Error: 15-52 pattern tile size must be non-zero
// Hint: 15-52 try setting the size manually
#line(stroke: pattern(path((0pt, 0pt), (1em, 0pt))))
