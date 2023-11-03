// Test the `circle` function.

---
// Default circle.
#box(circle())
#box(circle[Hey])

---
// Test auto sizing.
#set circle(inset: 0pt)

Auto-sized circle.
#circle(fill: rgb("eb5278"), stroke: 2pt + black,
  align(center + horizon)[But, soft!]
)

Center-aligned rect in auto-sized circle.
#circle(fill: forest, stroke: conifer,
  align(center + horizon,
    rect(fill: conifer, inset: 5pt)[But, soft!]
  )
)

Rect in auto-sized circle.
#circle(fill: forest,
  rect(fill: conifer, stroke: white, inset: 4pt)[
    #set text(8pt)
    But, soft! what light through yonder window breaks?
  ]
)

Expanded by height.
#circle(stroke: black, align(center)[A \ B \ C])

---
// Ensure circle directly in rect works.
#rect(width: 40pt, height: 30pt, fill: forest,
  circle(fill: conifer))

---
// Test relative sizing.
#set text(fill: white)
#show: rect.with(width: 100pt, height: 50pt, inset: 0pt, fill: rgb("aaa"))
#set align(center + horizon)
#stack(
  dir: ltr,
  spacing: 1fr,
  1fr,
  circle(radius: 10pt, fill: eastern, [A]),      // D=20pt
  circle(height: 60%, fill: eastern, [B]),       // D=30pt
  circle(width: 20% + 20pt, fill: eastern, [C]), // D=40pt
  1fr,
)

---
// Radius wins over width and height.
// Error: 23-34 unexpected argument: width
#circle(radius: 10pt, width: 50pt, height: 100pt, fill: eastern)
