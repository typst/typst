// Tests small patterns for pixel accuracy.

---
#box(
  width: 8pt,
  height: 1pt,
  fill: pattern((1pt, 1pt), square(size: 1pt, fill: black))
) \
#v(-1em)
#box(
  width: 8pt,
  height: 1pt,
  fill: pattern((2pt, 1pt), square(size: 1pt, fill: black))
)