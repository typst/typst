// Test polygons.

---
#set page(height: 220pt, width: 50pt)
#box({
  set polygon(stroke: 0.75pt, fill: blue)
  polygon((0em, 0pt))
  // this should not give an error
  polygon()
  polygon((0pt, 0pt), (10pt, 0pt))
  polygon((5pt, 0pt), (0pt, 10pt), (10pt, 10pt))
  polygon(
    (0pt, 0pt), (5pt, 5pt), (10pt, 0pt),
    (15pt, 5pt),
    (5pt, 10pt)
  )
  polygon(stroke: none, (5pt, 0pt), (0pt, 10pt), (10pt, 10pt))
  polygon(stroke: 3pt, fill: none, (5pt, 0pt), (0pt, 10pt), (10pt, 10pt))
  // relative size
  polygon((0pt, 0pt), (100%, 5pt), (50%, 10pt))
  // antiparallelogram
  polygon((0pt, 5pt), (5pt, 0pt), (0pt, 10pt), (5pt, 15pt))
  // self-intersections
  polygon((0pt, 10pt), (30pt, 20pt), (0pt, 30pt), (20pt, 0pt), (20pt, 35pt))
})

---
// Test errors.

// Error: 10-17 point array must contain exactly two entries
#polygon((50pt,))