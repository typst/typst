// Test the different radial gradient features.
---

#square(
  size: 100pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl),
)
---

#grid(
  columns: 2,
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (0%, 0%)),
  ),
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (0%, 100%)),
  ),
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (100%, 0%)),
  ),
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (100%, 100%)),
  ),
)

---

#square(
  size: 50pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl, radius: 10%),
)
#square(
  size: 50pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl, radius: 72%),
)

---
#circle(
  radius: 25pt,
  fill: gradient.radial(white, rgb("#8fbc8f"), focal-center: (35%, 35%), focal-radius: 5%),
)
#circle(
  radius: 25pt,
  fill: gradient.radial(white, rgb("#8fbc8f"), focal-center: (75%, 35%), focal-radius: 5%),
)
