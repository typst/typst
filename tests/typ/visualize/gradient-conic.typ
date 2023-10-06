// Test conic gradients

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsl),
)

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsl, center: (10%, 10%)),
)

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsl, center: (90%, 90%)),
)

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsl, angle: 90deg),
)