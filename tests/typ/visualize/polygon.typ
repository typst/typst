// Test polygons.

---
#set page(width: 50pt)
#set polygon(stroke: 0.75pt, fill: blue)

// These are not visible, but should also not give an error
#polygon()
#polygon((0em, 0pt))
#polygon((0pt, 0pt), (10pt, 0pt))
#polygon.regular(size: 0pt, vertices: 9)

#polygon((5pt, 0pt), (0pt, 10pt), (10pt, 10pt))
#polygon(
  (0pt, 0pt), (5pt, 5pt), (10pt, 0pt),
  (15pt, 5pt),
  (5pt, 10pt)
)
#polygon(stroke: none, (5pt, 0pt), (0pt, 10pt), (10pt, 10pt))
#polygon(stroke: 3pt, fill: none, (5pt, 0pt), (0pt, 10pt), (10pt, 10pt))

// Relative size
#polygon((0pt, 0pt), (100%, 5pt), (50%, 10pt))

// Antiparallelogram
#polygon((0pt, 5pt), (5pt, 0pt), (0pt, 10pt), (5pt, 15pt))

// Self-intersections
#polygon((0pt, 10pt), (30pt, 20pt), (0pt, 30pt), (20pt, 0pt), (20pt, 35pt))

// Regular polygon; should have equal side lengths
#for k in range(3, 9) {polygon.regular(size: 30pt, vertices: k,)}

---
// Error: 10-17 point array must contain exactly two entries
#polygon((50pt,))
