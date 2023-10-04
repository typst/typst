// Test sharp gradients.

---
#square(
  size: 100pt,
  fill: gradient.linear(..color.map.rainbow, space: color.hsl).sharp(10),
)

---
#square(
  size: 100pt,
  fill: gradient.linear(..color.map.rainbow, space: color.hsl).sharp(10, smoothness: 40%),
)
