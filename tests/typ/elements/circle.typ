// Test the `circle` function.

---
// Default circle.
#circle()
#circle[Hey]

---
// Test auto sizing.

Auto-sized circle. \
#circle(fill: rgb("eb5278"), thickness: 2pt,
  align(center, horizon)[But, soft!]
)

Center-aligned rect in auto-sized circle.
#circle(fill: forest, stroke: conifer,
  align(center, horizon,
    rect(fill: conifer, pad(5pt)[But, soft!])
  )
)

Rect in auto-sized circle. \
#circle(fill: forest,
  rect(fill: conifer, stroke: white, padding: 4pt)[
    #font(8pt)
    But, soft! what light through yonder window breaks?
  ]
)

Expanded by height.
#circle(stroke: black, align(center)[A \ B \ C])

---
// Ensure circle directly in rect works.
#rect(width: 40pt, height: 30pt, fill: forest, circle(fill: conifer))

---
// Test relative sizing.
#let centered(body) = align(center, horizon, body)
#font(fill: white)
#rect(width: 100pt, height: 50pt, fill: rgb("aaa"), centered[
  #circle(radius: 10pt, fill: eastern, centered[A])      // D=20pt
  #circle(height: 60%, fill: eastern, centered[B])       // D=30pt
  #circle(width: 20% + 20pt, fill: eastern, centered[C]) // D=40pt
])

---
// Radius wins over width and height.
// Error: 23-34 unexpected argument
#circle(radius: 10pt, width: 50pt, height: 100pt, fill: eastern)
