// Test conic gradients

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv),
)

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv, center: (10%, 10%)),
)

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv, center: (90%, 90%)),
)

---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv, angle: 90deg),
)
