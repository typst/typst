// Test repeated gradients.

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..gradient.inferno).repeat(2, mirror: true)
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..gradient.rainbow).repeat(2, mirror: true),
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..gradient.rainbow).repeat(5, mirror: true)
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..gradient.rainbow).sharp(10).repeat(5, mirror: false)
)

---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..gradient.rainbow).sharp(10).repeat(5, mirror: true)
)
