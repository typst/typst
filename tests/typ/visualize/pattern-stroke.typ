// Test pattern on strokes

---
#align(
  center + top,
  square(
    size: 50pt,
    stroke: 5pt + pattern(
      size: (5pt, 5pt),
      align(horizon + center, circle(fill: blue, radius: 2.5pt))
    )
  )
)
