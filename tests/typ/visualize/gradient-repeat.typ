// Test repeated gradients.

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.inferno).repeat(2, mirror: true)
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).repeat(2, mirror: true),
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).repeat(5, mirror: true)
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).sharp(10).repeat(5, mirror: false)
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).sharp(10).repeat(5, mirror: true)
)
