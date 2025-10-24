// Test polygons.

--- polygon render ---
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
#polygon(fill-rule: "non-zero", (0pt, 10pt), (30pt, 20pt), (0pt, 30pt), (20pt, 0pt), (20pt, 35pt))
#polygon(fill-rule: "even-odd", (0pt, 10pt), (30pt, 20pt), (0pt, 30pt), (20pt, 0pt), (20pt, 35pt))

// Regular polygon; should have equal side lengths
#for k in range(3, 9) {polygon.regular(size: 30pt, vertices: k,)}

--- polygon-line-join render ---
// Line joins
#stack(
  dir: ltr,
  spacing: 1em,
  polygon(stroke: (thickness: 4pt, paint: blue, join: "round"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  polygon(stroke: (thickness: 4pt, paint: blue, join: "bevel"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  polygon(stroke: (thickness: 4pt, paint: blue, join: "miter"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  polygon(stroke: (thickness: 4pt, paint: blue, join: "miter", miter-limit: 20.0),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
)

--- polygon-bad-point-array render ---
// Error: 10-17 array must contain exactly two items
// Hint: 10-17 the first item determines the value for the X axis and the second item the value for the Y axis
#polygon((50pt,))

--- polygon-infinite-size render ---
// Error: 2-57 cannot create polygon with infinite size
#polygon((0pt, 0pt), (0pt, 1pt), (float.inf * 1pt, 0pt))
