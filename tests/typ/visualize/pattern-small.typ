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
